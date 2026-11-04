use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::domain::{
    error::AppError,
    model::{CreateItem, Item, UpdateItem},
    port::ItemRepository,
};

// ─── Health Check ────────────────────────────────────────────

/// 헬스체크 엔드포인트 (Liveness / Readiness Probe)
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

// ─── CRUD Handlers (ItemRepository Port 기반) ────────────────

/// 전체 아이템 조회
pub async fn list_items<R: ItemRepository>(
    State(repo): State<R>,
) -> Result<Json<Vec<Item>>, AppError> {
    let items = repo.find_all().await?;
    Ok(Json(items))
}

/// 단일 아이템 조회
pub async fn get_item<R: ItemRepository>(
    State(repo): State<R>,
    Path(id): Path<Uuid>,
) -> Result<Json<Item>, AppError> {
    repo.find_by_id(id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Item with id '{}' not found", id)))
}

/// 아이템 생성
pub async fn create_item<R: ItemRepository>(
    State(repo): State<R>,
    Json(input): Json<CreateItem>,
) -> Result<(StatusCode, Json<Item>), AppError> {
    let item = repo.create(input).await?;
    tracing::info!(id = %item.id, "Item created");
    Ok((StatusCode::CREATED, Json(item)))
}

/// 아이템 수정 (부분 업데이트)
pub async fn update_item<R: ItemRepository>(
    State(repo): State<R>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateItem>,
) -> Result<Json<Item>, AppError> {
    let item = repo.update(id, input).await?;
    tracing::info!(id = %id, "Item updated");
    Ok(Json(item))
}

/// 아이템 삭제
pub async fn delete_item<R: ItemRepository>(
    State(repo): State<R>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo.delete(id).await?;
    tracing::info!(id = %id, "Item deleted");
    Ok(StatusCode::NO_CONTENT)
}
