



cargo run --release -- \
  --worker-urls http://127.0.0.1:8000 \
  --policy round_robin \
  --intra-node-data-parallel-size 1 \
  --host 127.0.0.1 \
  --port 8090


curl -X POST http://localhost:8090/v1/chat/completions \
  -H "X-Session-ID: my-session-123" \
  -H "Content-Type: application/json" \
  -d '{"model": "llama-3", "messages": [{"role": "user", "content": "Hello!"}]}'