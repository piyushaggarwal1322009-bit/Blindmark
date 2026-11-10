from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

P = 2**61 - 1

app = FastAPI()

state = {
    "total": 0,
    "count": 0
}

class SharePayload(BaseModel):
    share: int

@app.post("/submit")
def submit(payload: SharePayload):
    if not (0 <= payload.share < P):
        raise HTTPException(status_code=400, detail="Invalid share value")
    state["total"] = (state["total"] + payload.share) % P
    state["count"] += 1
    return {"ok": True}

@app.get("/total")
def get_total():
    return {"total": state["total"], "count": state["count"]}

