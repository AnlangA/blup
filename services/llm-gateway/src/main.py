import logging
import structlog
import uvicorn
from fastapi import FastAPI

from src.config import settings
from src.routes import complete, health

structlog.configure(
    processors=[
        structlog.stdlib.add_log_level,
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.JSONRenderer(),
    ],
    wrapper_class=structlog.make_filtering_bound_logger(logging.INFO),
)

app = FastAPI(title="Blup LLM Gateway", version="0.1.0")
app.include_router(health.router)
app.include_router(complete.router)

if __name__ == "__main__":
    uvicorn.run(app, host=settings.host, port=settings.port, log_level="info")
