use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::config::AppConfig;
use crate::domain::error::AppError;

/// ALLOWED_ORIGINS 설정을 기반으로 최소 권한 CORS 레이어 생성
pub fn build_cors_layer(config: &AppConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .allowed_origins
        .iter()
        .filter_map(|s| HeaderValue::from_str(s.trim()).ok())
        .collect();

    let allow_origin = if origins.is_empty() {
        AllowOrigin::list([
            HeaderValue::from_static("http://localhost:3000"),
            HeaderValue::from_static("http://localhost:8080"),
        ])
    } else {
        AllowOrigin::list(origins)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-api-key"),
        ])
}

/// 상수 시간(Constant-time) 바이트 비교로 타이밍 공격 방지
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// 쓰기 작업(POST/PUT/DELETE) 시 API_KEY가 설정된 경우 인증 검증
pub async fn api_key_auth(
    State(expected_key): State<Option<Arc<str>>>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if matches!(*req.method(), Method::POST | Method::PUT | Method::DELETE) {
        if let Some(expected) = expected_key.as_deref() {
            if !expected.is_empty() {
                let provided = req
                    .headers()
                    .get("x-api-key")
                    .and_then(|v| v.to_str().ok())
                    .or_else(|| {
                        req.headers()
                            .get(header::AUTHORIZATION)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.strip_prefix("Bearer "))
                    });

                let is_valid = provided
                    .map(|p| constant_time_eq(p.as_bytes(), expected.as_bytes()))
                    .unwrap_or(false);

                if !is_valid {
                    return Err(AppError::Unauthorized(
                        "Invalid or missing API key".to_string(),
                    ));
                }
            }
        }
    }
    Ok(next.run(req).await)
}
