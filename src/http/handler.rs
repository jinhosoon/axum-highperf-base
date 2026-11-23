use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use jiff::Timestamp;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::{
    error::AppError,
    model::{CreateItem, Item, ScoredItem, SearchQuery, UpdateItem},
};
use crate::store::ItemRepo;

/// 검색 쿼리 파라미터 DTO (`GET /api/items/search?q=...&limit=...`)
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<usize>,
}

// ─── Health Check ────────────────────────────────────────────

/// 헬스체크 엔드포인트 (Liveness / Readiness Probe)
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": Timestamp::now().to_string(),
    }))
}

// ─── CRUD & Search Handlers (`enum ItemRepo` 닫힌 다형성) ────

/// 전체 아이템 조회
pub async fn list_items(State(repo): State<ItemRepo>) -> Result<Json<Vec<Item>>, AppError> {
    let items = repo.find_all().await?;
    Ok(Json(items))
}

/// 역색인 + Okapi BM25 랭킹 검색
pub async fn search_items(
    State(repo): State<ItemRepo>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<ScoredItem>>, AppError> {
    let query = SearchQuery::try_new(&params.q, params.limit)?;
    let results = repo.search(query).await?;
    Ok(Json(results))
}

/// 단일 아이템 조회
pub async fn get_item(
    State(repo): State<ItemRepo>,
    Path(id): Path<Uuid>,
) -> Result<Json<Item>, AppError> {
    repo.find_by_id(id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Item with id '{}' not found", id)))
}

/// 아이템 생성
pub async fn create_item(
    State(repo): State<ItemRepo>,
    Json(input): Json<CreateItem>,
) -> Result<(StatusCode, Json<Item>), AppError> {
    let item = repo.create(input).await?;
    tracing::info!(id = %item.id, "Item created");
    Ok((StatusCode::CREATED, Json(item)))
}

/// 아이템 수정 (부분 업데이트)
pub async fn update_item(
    State(repo): State<ItemRepo>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateItem>,
) -> Result<Json<Item>, AppError> {
    let item = repo.update(id, input).await?;
    tracing::info!(id = %id, "Item updated");
    Ok(Json(item))
}

/// 아이템 삭제
pub async fn delete_item(
    State(repo): State<ItemRepo>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo.delete(id).await?;
    tracing::info!(id = %id, "Item deleted");
    Ok(StatusCode::NO_CONTENT)
}
