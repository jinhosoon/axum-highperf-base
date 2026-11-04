# ──────────────────────────────────────────────
# Stage 1: Build
# ──────────────────────────────────────────────
FROM rust:1-slim AS builder

WORKDIR /app

# 의존성 캐싱: Cargo.toml/lock만 먼저 복사하여 레이어 캐시 활용
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && \
    echo "" > src/lib.rs && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# 실제 소스 복사 후 빌드
COPY src ./src
RUN touch src/lib.rs src/main.rs && cargo build --release

# ──────────────────────────────────────────────
# Stage 2: Runtime (경량 + 보안)
# ──────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# 최소 런타임 의존성만 설치, 캐시 제거
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# 비 root 사용자 생성
RUN groupadd --gid 10001 appgroup && \
    useradd --uid 10001 --gid appgroup --shell /bin/false --create-home appuser

# 바이너리 복사 (실행 권한만)
COPY --from=builder --chmod=555 /app/target/release/rust-api /usr/local/bin/rust-api

USER appuser
EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=3s --retries=3 \
    CMD ["rust-api", "--healthcheck"]

ENTRYPOINT ["rust-api"]
