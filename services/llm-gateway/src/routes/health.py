from fastapi import APIRouter

router = APIRouter()


@router.get("/health")
async def health():
    return {"status": "ok", "version": "0.1.0"}


@router.get("/health/providers")
async def provider_health():
    from src.routes.complete import providers

    return {"providers": [{"name": p.provider_name()} for p in providers]}
