use std::sync::Arc;

/// 애플리케이션 런타임 설정
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub allowed_origins: Vec<String>,
    pub api_key: Option<Arc<str>>,
    pub max_concurrency: usize,
    pub request_timeout_secs: u64,
}

impl AppConfig {
    /// 환경변수에서 설정을 1회 로드 (요청마다 `std::env::var` 시스템 콜을 호출하지 않도록 캐싱)
    pub fn from_env() -> Self {
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let allowed_origins = std::env::var("ALLOWED_ORIGINS")
            .ok()
            .map(|val| {
                val.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let api_key = std::env::var("API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .map(Arc::from);

        Self {
            host,
            port,
            allowed_origins,
            api_key,
            max_concurrency: 1024,
            request_timeout_secs: 10,
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
