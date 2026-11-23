use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::{body::Body, http::Request};
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use rust_api::adapters::outbound::memory_repo::NUM_SHARDS;
use rust_api::domain::model::{CreateItem, Item, SearchQuery};
use rust_api::domain::port::ItemRepository;
use rust_api::{create_router, AppConfig, InMemoryItemRepository, ItemRepo};

const DOC_COUNT: usize = 10_000;
const SEARCH_ITERS: usize = 500;
const CONCURRENCY_TASKS: usize = 32;
const OPS_PER_TASK: usize = 2_000;
const HTTP_ITERS: usize = 1_000;

fn percentile(sorted_us: &[f64], p: f64) -> f64 {
    if sorted_us.is_empty() {
        return 0.0;
    }
    let idx = ((sorted_us.len() as f64 - 1.0) * p).round() as usize;
    sorted_us[idx.min(sorted_us.len() - 1)]
}

fn generate_paper(i: usize) -> CreateItem {
    let domains = [
        "molecular dynamics",
        "protein folding",
        "catalyst synthesis",
        "battery electrolyte",
        "photovoltaic perovskite",
        "genomic sequencing",
        "neural operator",
        "fluid turbulence",
    ];
    let methods = [
        "transformer attention",
        "bayesian optimization",
        "monte carlo sampling",
        "graph neural network",
        "density functional theory",
        "reinforcement learning",
        "variational inference",
        "spectral clustering",
    ];
    let d = domains[i % domains.len()];
    let m = methods[(i / domains.len()) % methods.len()];

    let rare_tag = if i.is_multiple_of(250) {
        " quantum-annealing superconductor"
    } else {
        ""
    };

    CreateItem {
        title: format!("Paper #{}: {} via {}{}", i, d, m, rare_tag),
        content: format!(
            "Autonomous science study {} investigating {} with {} on high-throughput laboratory pipelines. \
             Benchmarking memory efficiency, latency bounds, and reproducible verification across {} trials.{}",
            i,
            d,
            m,
            (i % 100) + 10,
            rare_tag
        ),
    }
}

/// Baseline 비교용 단순 선형 전수 탐색 (O(N))
fn linear_scan_search(items: &[Item], query_terms: &[&str], limit: usize) -> Vec<Uuid> {
    let mut hits: Vec<(Uuid, usize)> = Vec::new();
    for item in items {
        let title_lower = item.title.as_str().to_lowercase();
        let content_lower = item.content.as_str().to_lowercase();
        let mut score = 0usize;
        for term in query_terms {
            if title_lower.contains(term) {
                score += 2;
            }
            if content_lower.contains(term) {
                score += 1;
            }
        }
        if score > 0 {
            hits.push((item.id, score));
        }
    }
    hits.sort_by(|a, b| b.1.cmp(&a.1));
    hits.truncate(limit);
    hits.into_iter().map(|(id, _)| id).collect()
}

#[tokio::main]
async fn main() {
    println!("================================================================");
    println!("  axum-highperf-base Benchmark Suite (Release Build)");
    println!("================================================================\n");

    // ── 데이터셋 10,000건 적재 ──
    let repo = InMemoryItemRepository::new();
    let mut raw_items = Vec::with_capacity(DOC_COUNT);
    let ingest_start = Instant::now();
    for i in 0..DOC_COUNT {
        let item = repo.create(generate_paper(i)).await.unwrap();
        raw_items.push(item);
    }
    let ingest_ms = ingest_start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "[Setup] Indexed {} scientific documents in {:.1} ms ({:.0} docs/sec)\n",
        DOC_COUNT,
        ingest_ms,
        DOC_COUNT as f64 / (ingest_ms / 1000.0)
    );

    // ── Benchmark 1: Linear Scan vs Inverted Index + Okapi BM25 ──
    let query_str = "quantum superconductor catalyst";
    let query_terms = ["quantum", "superconductor", "catalyst"];
    let search_query = SearchQuery::try_new(query_str, Some(10)).unwrap();

    let mut linear_latencies = Vec::with_capacity(SEARCH_ITERS);
    let linear_start = Instant::now();
    for _ in 0..SEARCH_ITERS {
        let t0 = Instant::now();
        let res = linear_scan_search(&raw_items, &query_terms, 10);
        std::hint::black_box(res);
        linear_latencies.push(t0.elapsed().as_secs_f64() * 1_000_000.0);
    }
    let linear_total = linear_start.elapsed().as_secs_f64();
    linear_latencies.sort_by(|a, b| a.total_cmp(b));

    let mut bm25_latencies = Vec::with_capacity(SEARCH_ITERS);
    let bm25_start = Instant::now();
    for _ in 0..SEARCH_ITERS {
        let t0 = Instant::now();
        let res = repo.search(search_query.clone()).await.unwrap();
        std::hint::black_box(res);
        bm25_latencies.push(t0.elapsed().as_secs_f64() * 1_000_000.0);
    }
    let bm25_total = bm25_start.elapsed().as_secs_f64();
    bm25_latencies.sort_by(|a, b| a.total_cmp(b));

    let linear_qps = SEARCH_ITERS as f64 / linear_total;
    let bm25_qps = SEARCH_ITERS as f64 / bm25_total;
    println!("--- Benchmark 1: Search (10,000 docs, {} queries) ---", SEARCH_ITERS);
    println!(
        "  Linear Scan (O(N))         : {:>9.1} QPS | p50={:>7.1} us | p95={:>7.1} us | p99={:>7.1} us",
        linear_qps,
        percentile(&linear_latencies, 0.50),
        percentile(&linear_latencies, 0.95),
        percentile(&linear_latencies, 0.99),
    );
    println!(
        "  Inverted Index + BM25      : {:>9.1} QPS | p50={:>7.1} us | p95={:>7.1} us | p99={:>7.1} us",
        bm25_qps,
        percentile(&bm25_latencies, 0.50),
        percentile(&bm25_latencies, 0.95),
        percentile(&bm25_latencies, 0.99),
    );
    println!("  Speedup                    : {:.1}x faster\n", bm25_qps / linear_qps);

    // ── Benchmark 2: Single RwLock<HashMap> vs 64-Shard RwLock (80% Read / 20% Write) ──
    let seed_ids: Arc<Vec<Uuid>> = Arc::new(raw_items.iter().take(1024).map(|i| i.id).collect());
    let sample_item = Arc::new(raw_items[0].clone());

    // 2a. Single global RwLock
    let single_map: Arc<RwLock<HashMap<Uuid, Item>>> = Arc::new(RwLock::new(
        raw_items
            .iter()
            .take(1024)
            .map(|i| (i.id, i.clone()))
            .collect(),
    ));
    let single_start = Instant::now();
    let mut handles = Vec::with_capacity(CONCURRENCY_TASKS);
    for t in 0..CONCURRENCY_TASKS {
        let map = single_map.clone();
        let ids = seed_ids.clone();
        let item = sample_item.clone();
        handles.push(tokio::spawn(async move {
            for op in 0..OPS_PER_TASK {
                let id = ids[(t * 31 + op) % ids.len()];
                if op.is_multiple_of(5) {
                    let mut guard = map.write().await;
                    guard.insert(id, (*item).clone());
                } else {
                    let guard = map.read().await;
                    std::hint::black_box(guard.get(&id));
                }
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    let single_ops = (CONCURRENCY_TASKS * OPS_PER_TASK) as f64 / single_start.elapsed().as_secs_f64();

    // 2b. 64-Shard RwLock
    let sharded_map: Arc<[RwLock<HashMap<Uuid, Item>>; NUM_SHARDS]> =
        Arc::new(std::array::from_fn(|_| RwLock::new(HashMap::new())));
    for item in raw_items.iter().take(1024) {
        let idx = (item.id.as_bytes()[0] as usize) & (NUM_SHARDS - 1);
        sharded_map[idx].write().await.insert(item.id, item.clone());
    }
    let sharded_start = Instant::now();
    let mut shard_handles = Vec::with_capacity(CONCURRENCY_TASKS);
    for t in 0..CONCURRENCY_TASKS {
        let shards = sharded_map.clone();
        let ids = seed_ids.clone();
        let item = sample_item.clone();
        shard_handles.push(tokio::spawn(async move {
            for op in 0..OPS_PER_TASK {
                let id = ids[(t * 31 + op) % ids.len()];
                let idx = (id.as_bytes()[0] as usize) & (NUM_SHARDS - 1);
                if op.is_multiple_of(5) {
                    let mut guard = shards[idx].write().await;
                    guard.insert(id, (*item).clone());
                } else {
                    let guard = shards[idx].read().await;
                    std::hint::black_box(guard.get(&id));
                }
            }
        }));
    }
    for h in shard_handles {
        h.await.unwrap();
    }
    let sharded_ops =
        (CONCURRENCY_TASKS * OPS_PER_TASK) as f64 / sharded_start.elapsed().as_secs_f64();

    println!(
        "--- Benchmark 2: Concurrency ({} tasks x {} ops, 80% R / 20% W) ---",
        CONCURRENCY_TASKS, OPS_PER_TASK
    );
    println!("  Single RwLock<HashMap>     : {:>10.0} ops/sec", single_ops);
    println!("  64-Shard RwLock<HashMap>   : {:>10.0} ops/sec", sharded_ops);
    println!(
        "  Speedup                    : {:.2}x higher throughput\n",
        sharded_ops / single_ops
    );

    // ── Benchmark 3: HTTP End-to-End (Axum Router + Tower Middleware Stack) ──
    let config = AppConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        allowed_origins: vec![],
        api_key: None,
        max_concurrency: 1024,
        request_timeout_secs: 30,
    };
    let app = create_router(ItemRepo::from(repo), &config);

    // 3a. /health
    let mut health_lat = Vec::with_capacity(HTTP_ITERS);
    let h_start = Instant::now();
    for _ in 0..HTTP_ITERS {
        let t0 = Instant::now();
        let res = app
            .clone()
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        std::hint::black_box(res.status());
        health_lat.push(t0.elapsed().as_secs_f64() * 1_000_000.0);
    }
    let h_rps = HTTP_ITERS as f64 / h_start.elapsed().as_secs_f64();
    health_lat.sort_by(|a, b| a.total_cmp(b));

    // 3b. /api/items/search (10,000 docs)
    let mut search_http_lat = Vec::with_capacity(HTTP_ITERS);
    let s_start = Instant::now();
    for _ in 0..HTTP_ITERS {
        let t0 = Instant::now();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/items/search?q=quantum%20superconductor%20catalyst&limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        std::hint::black_box(res.status());
        search_http_lat.push(t0.elapsed().as_secs_f64() * 1_000_000.0);
    }
    let s_rps = HTTP_ITERS as f64 / s_start.elapsed().as_secs_f64();
    search_http_lat.sort_by(|a, b| a.total_cmp(b));

    println!("--- Benchmark 3: HTTP Router E2E ({} requests) ---", HTTP_ITERS);
    println!(
        "  GET /health                : {:>9.1} RPS | p50={:>7.1} us | p95={:>7.1} us | p99={:>7.1} us",
        h_rps,
        percentile(&health_lat, 0.50),
        percentile(&health_lat, 0.95),
        percentile(&health_lat, 0.99),
    );
    println!(
        "  GET /api/items/search      : {:>9.1} RPS | p50={:>7.1} us | p95={:>7.1} us | p99={:>7.1} us",
        s_rps,
        percentile(&search_http_lat, 0.50),
        percentile(&search_http_lat, 0.95),
        percentile(&search_http_lat, 0.99),
    );
}
