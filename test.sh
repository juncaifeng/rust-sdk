#!/bin/bash

# generate a test token (requires a simple python script or just an invalid token to test rejection)
# We can just test that GET /v1/tools without auth is accepted if require_auth is false

echo "Testing GET /v1/tools..."
curl -v http://127.0.0.1:8080/v1/tools

echo -e "\n\nTesting POST /v1/tools/call..."
curl -v -X POST http://127.0.0.1:8080/v1/tools/call \
  -H "Content-Type: application/json" \
  -d '{"name": "echo", "arguments": {"message": "Hello from Gateway!"}}'
