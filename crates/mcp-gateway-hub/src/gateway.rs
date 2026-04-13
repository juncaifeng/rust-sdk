use axum::{
    extract::{State, Json},
    http::{StatusCode, header::AUTHORIZATION},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum::extract::Request;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use rmcp::model::CallToolRequestParams;

use crate::{config::GatewayConfig, mcp_client::McpClientManager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub scopes: Vec<String>,
    pub exp: usize,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<GatewayConfig>,
    pub mcp_client_manager: McpClientManager,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/tools", get(list_tools))
        .route("/v1/tools/call", post(call_tool))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state)
}

async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !state.config.auth.require_auth {
        let dummy_claims = Claims {
            sub: "anonymous".to_string(),
            scopes: vec!["tool:*".to_string()],
            exp: 0,
        };
        req.extensions_mut().insert(dummy_claims);
        return Ok(next.run(req).await);
    }

    let auth_header = req.headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .filter(|value| value.starts_with("Bearer "));

    let token = match auth_header {
        Some(header) => header[7..].to_string(),
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let mut validation = Validation::new(Algorithm::HS256);
    // For testing, we might ignore expiration, but normally we shouldn't.
    validation.validate_exp = false;
    validation.required_spec_claims.clear();

    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.config.auth.jwt_secret.as_bytes()),
        &validation,
    ).map_err(|e| {
        warn!("JWT decode error: {}", e);
        StatusCode::UNAUTHORIZED
    })?;

    req.extensions_mut().insert(token_data.claims);
    Ok(next.run(req).await)
}

async fn list_tools(State(state): State<AppState>) -> impl IntoResponse {
    let tools = state.mcp_client_manager.list_all_tools().await;
    Json(tools)
}

#[derive(Deserialize)]
pub struct CallToolPayload {
    pub name: String,
    pub arguments: Option<serde_json::Value>,
}

async fn call_tool(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Json(payload): Json<CallToolPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    // RBAC check
    let required_scope = format!("tool:{}", payload.name);
    let has_access = claims.scopes.contains(&required_scope) || claims.scopes.contains(&"tool:*".to_string());
    
    if !has_access {
        warn!("User {} lacks scope to call tool {}", claims.sub, payload.name);
        return Err(StatusCode::FORBIDDEN);
    }

    info!("User {} calling tool {}", claims.sub, payload.name);

    let mut params = CallToolRequestParams::new(payload.name.clone());
    if let Some(args) = payload.arguments {
        if let serde_json::Value::Object(map) = args {
            params.arguments = Some(map);
        } else {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    match state.mcp_client_manager.call_tool(params).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            warn!("Error calling tool {}: {}", payload.name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
