use crate::config::ServerProcessConfig;
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;

pub struct DaemonManager;

impl DaemonManager {
    pub fn spawn(config: &ServerProcessConfig) -> anyhow::Result<TokioChildProcess> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);
        
        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let process = TokioChildProcess::new(cmd)?;
        Ok(process)
    }
}
