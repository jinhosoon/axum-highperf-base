use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tower::ServiceExt;

use rust_api::{create_router, run_healthcheck, AppConfig, ItemRepo};

fn test_config(api_key: Option<&str>) -> AppConfig {
    AppConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        allowed_origins: vec![],
        api_key: api_key.map(Arc::from),
        max_concurrency: 128,
        request_timeout_secs: 5,
    }
}

async fn read_json(body: Body) -> Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn healthcheck_probe_and_router_work_end_to_end() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let mut config = test_config(Some("test-secret-key"));
    config.port = port;

    let app = create_router(ItemRepo::in_memory(), &config);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // CLI 어댑터의 자체 TCP 헬스체크 프로브가 실제 서버를 대상으로 성공하는지 검증
    let healthy = tokio::task::spawn_blocking(move || run_healthcheck(port))
        .await
        .unwrap();

    assert!(
        healthy,
        "Expected healthcheck probe against port {} to succeed",
        port
    );
}

#[tokio::test]
async fn api_key_auth_enforces_constant_time_check() {
    let app = create_router(ItemRepo::in_memory(), &test_config(Some("secret-123")));
    let payload = json!({"title": "Auth Test", "content": "Protected write"}).to_string();

    // 1. 읽기(GET)는 공개 접근 허용 -> 200
    let get_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);

    // 2. 쓰기(POST) 시 키 누락 -> 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/items")
                .header("content-type", "application/json")
                .body(Body::from(payload.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 3. 쓰기(POST) 시 잘못된 키 -> 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/items")
                .header("content-type", "application/json")
                .header("x-api-key", "wrong-key")
                .body(Body::from(payload.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 4. 쓰기(POST) 시 올바른 x-api-key -> 201
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/items")
                .header("content-type", "application/json")
                .header("x-api-key", "secret-123")
                .body(Body::from(payload.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 5. 쓰기(POST) 시 올바른 Bearer 토큰 -> 201
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/items")
                .header("content-type", "application/json")
                .header("authorization", "Bearer secret-123")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn crud_and_bm25_search_pipeline_work_end_to_end() {
    let app = create_router(ItemRepo::in_memory(), &test_config(None));

    // 1. 빈 제목은 Newtype 검증으로 400 Bad Request
    let bad_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/items")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"title": "   ", "content": "invalid"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_res.status(), StatusCode::BAD_REQUEST);

    // 2. 검색 테스트용 문서 3건 생성
    let docs = [
        (
            "Autonomous Science Rust Search Engine",
            "Inverted index and Okapi BM25 ranking for million-scale scientific papers",
        ),
        (
            "General Backend Notes",
            "Notes mentioning rust once in the body without search focus",
        ),
        (
            "Kubernetes GitOps with Argo CD",
            "Zero-downtime rolling updates and HPA autoscaling",
        ),
    ];

    let mut created_ids = Vec::new();
    for (title, content) in docs {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/items")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"title": title, "content": content}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = read_json(res.into_body()).await;
        created_ids.push(body["id"].as_str().unwrap().to_string());
    }

    // 3. BM25 검색 (`GET /api/items/search?q=rust+bm25&limit=5`)
    let search_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/items/search?q=rust%20bm25&limit=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_res.status(), StatusCode::OK);
    let hits = read_json(search_res.into_body()).await;
    let hits_arr = hits.as_array().unwrap();
    assert_eq!(hits_arr.len(), 2);
    // 제목 + 본문 모두 매칭된 1번 문서가 1순위여야 함
    assert_eq!(hits_arr[0]["item"]["id"].as_str().unwrap(), created_ids[0]);
    assert!(hits_arr[0]["score"].as_f64().unwrap() > hits_arr[1]["score"].as_f64().unwrap());

    // 4. 수정(PUT) 후 역색인 갱신 확인
    let update_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/items/{}", created_ids[1]))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"title": "Quantum Chemistry Pipeline"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_res.status(), StatusCode::OK);

    let updated_search = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/items/search?q=quantum")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated_search.status(), StatusCode::OK);
    let updated_hits = read_json(updated_search.into_body()).await;
    assert_eq!(updated_hits.as_array().unwrap().len(), 1);
    assert_eq!(
        updated_hits[0]["item"]["id"].as_str().unwrap(),
        created_ids[1]
    );

    // 5. 삭제(DELETE) 후 404 및 검색 결과 제외 확인
    let del_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/items/{}", created_ids[1]))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_res.status(), StatusCode::NO_CONTENT);

    let after_del = app
        .oneshot(
            Request::builder()
                .uri("/api/items/search?q=quantum")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let after_del_hits = read_json(after_del.into_body()).await;
    assert!(after_del_hits.as_array().unwrap().is_empty());
}
