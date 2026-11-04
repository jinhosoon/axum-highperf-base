use rust_api::{
    create_router, run_healthcheck, telemetry, AppConfig, InMemoryItemRepository,
};

#[tokio::main]
async fn main() {
    let config = AppConfig::from_env();

    // ── 컨테이너 HEALTHCHECK 모드 (`rust-api --healthcheck`) ──
    if std::env::args().any(|arg| arg == "--healthcheck") {
        std::process::exit(if run_healthcheck(config.port) { 0 } else { 1 });
    }

    // ── 로깅 초기화 ──
    telemetry::init_tracing();

    // ── 의존성 주입 (Outbound Adapter → Inbound HTTP Router) ──
    let repository = InMemoryItemRepository::new();
    let app = create_router(repository, &config);

    // ── 서버 시작 & Graceful Shutdown ──
    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("Failed to bind TCP listener");
    tracing::info!("🚀 Server listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(telemetry::shutdown_signal())
        .await
        .expect("Server exited with error");
}
