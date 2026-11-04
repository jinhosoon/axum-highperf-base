/// 도메인 및 애플리케이션 공통 에러 타입
#[derive(Debug, Clone)]
pub enum AppError {
    /// 리소스를 찾을 수 없음 (404)
    NotFound(String),
    /// 도메인 검증 실패 및 잘못된 요청 (400)
    BadRequest(String),
    /// 인증 실패 (401)
    Unauthorized(String),
    /// 저장 용량 초과 등 요청 제한 (429)
    TooManyRequests(String),
}
