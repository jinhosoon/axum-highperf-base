use thiserror::Error;

/// 도메인 및 애플리케이션 공통 에러 타입
///
/// `thiserror`를 통해 `std::error::Error`와 `Display`를 구현하여
/// 표준 Rust 에러 생태계 및 `?` 연산자 변환과 자연스럽게 호환됩니다.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AppError {
    /// 리소스를 찾을 수 없음 (404)
    #[error("{0}")]
    NotFound(String),
    /// 도메인 검증 실패 및 잘못된 요청 (400)
    #[error("{0}")]
    BadRequest(String),
    /// 인증 실패 (401)
    #[error("{0}")]
    Unauthorized(String),
    /// 저장 용량 초과 등 요청 제한 (429)
    #[error("{0}")]
    TooManyRequests(String),
}
