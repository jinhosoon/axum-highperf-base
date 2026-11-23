pub mod error;
pub mod handler;
pub mod middleware;

use axum::{
    error_handling::HandleErrorLayer,
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware::from_fn_with_state,
    routing::get,
    BoxError, Router,
};
use std::time::Duration;
use tower::{limit::ConcurrencyLimitLayer, timeout::TimeoutLayer, ServiceBuilder};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

use crate::adapters::outbound::ItemRepo;
use crate::config::AppConfig;
use handler::{
    create_item, delete_item, get_item, health_check, list_items, search_items, update_item,
};
use middleware::{api_key_auth, build_cors_layer};

/// 최대 요청 본문 크기 (64 KiB)
pub const MAX_BODY_SIZE: usize = 64 * 1024;

/// Inbound HTTP Adapter: 라우터 및 고성능/보안 미들웨어 스택 조립
///
/// `ItemRepo` 닫힌 `enum`을 상태로 주입하여 제네릭(`<R>`) 및 터보피쉬(`::<R>`) 없이
/// 정적 디스패치 라우터를 구성합니다.
pub fn create_router(repo: impl Into<ItemRepo>, config: &AppConfig) -> Router {
    let cors = build_cors_layer(config);

    let api_routes = Router::new()
        .route("/api/items", get(list_items).post(create_item))
        .route("/api/items/search", get(search_items))
        .route(
            "/api/items/{id}",
            get(get_item).put(update_item).delete(delete_item),
        )
        .route_layer(from_fn_with_state(config.api_key.clone(), api_key_auth));

    Router::new()
        .route("/health", get(health_check))
        .merge(api_routes)
        .with_state(repo.into())
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .layer(CompressionLayer::new())
                .layer(ConcurrencyLimitLayer::new(config.max_concurrency))
                .layer(HandleErrorLayer::new(|_: BoxError| async {
                    StatusCode::REQUEST_TIMEOUT
                }))
                .layer(TimeoutLayer::new(Duration::from_secs(
                    config.request_timeout_secs,
                ))),
        )
}
