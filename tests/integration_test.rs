use std::sync::Arc;
use tokio::net::TcpListener;

use rust_api::{create_router, run_healthcheck, AppConfig, InMemoryItemRepository};

#[tokio::test]
async fn healthcheck_probe_and_router_work_end_to_end() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let config = AppConfig {
        host: "127.0.0.1".to_string(),
        port,
        allowed_origins: vec![],
        api_key: Some(Arc::from("test-secret-key")),
        max_concurrency: 128,
        request_timeout_secs: 5,
    };

    let repo = InMemoryItemRepository::new();
    let app = create_router(repo, &config);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // CLI 어댑터의 자체 TCP 헬스체크 프로브가 실제 서버를 대상으로 성공하는지 검증
    let healthy = tokio::task::spawn_blocking(move || run_healthcheck(port))
        .await
        .unwrap();

    assert!(healthy, "Expected healthcheck probe against port {} to succeed", port);
}
