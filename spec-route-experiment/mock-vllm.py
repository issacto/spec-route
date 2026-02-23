from fastapi import FastAPI
from pydantic import BaseModel
from typing import List
import time
import subprocess
import os
from threading import Thread
import sys
from fastapi import BackgroundTasks
import signal


app = FastAPI()
status = True
class ChatMessage(BaseModel):
    role: str
    content: str

class ChatRequest(BaseModel):
    model: str
    messages: List[ChatMessage]

@app.get("/health")
def health():
    return {"status": "ok" if status else "not ok" }

@app.post("/v1/chat/completions")
async def chat_completions(req: ChatRequest):
    return {
        "id": "mock-id",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": req.model,
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": "Hello! This is a mock reply."},
                "finish_reason": "stop"
            }
        ]
}

def kill():
    os.killpg(os.getpgid(os.getpid()), signal.SIGINT)

@app.post("/restart")
def restart(background_tasks: BackgroundTasks):
    background_tasks.add_task(kill)
    return {"status": "restarting"}

# Optional: only needed if you want to run without uvicorn command
if __name__ == "__main__":
    import uvicorn
    uvicorn.run("mock-vllm:app", host="127.0.0.1", port=8000, reload=True)