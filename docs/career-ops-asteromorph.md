# 🎯 Asteromorph (아스테로모프) Software Engineer — `career-ops` 산출물 패키지

> 대상 공고: 주식회사 아스테로모프 (Asteromorph) — Software Engineer (`https://careers.asteromorph.com/job_posting/pGqHMndu`)  
> 근거 리포지토리: `https://github.com/jinhosoon/axum-highperf-base`  
> 모든 성능 수치는 `cargo run --release --example bench`로 Arch Linux에서 실측한 재현 가능 데이터입니다 (`career-ops` Fact-Check Gate 통과).

---

## 1. Block H — 아스테로모프 지원 동기 초안

자율 과학(Autonomous Science) AI 에이전트가 수백만 편의 논문과 실험 데이터를 고속으로 탐색하고 가설을 검증하려면, 화려한 프레임워크보다 **타입 안전성으로 복잡성을 제어하고 프로파일링 근거에 기반해 병목을 제거하는 Rust 시스템 백엔드**가 핵심이라고 생각합니다.

저는 이 문제를 직접 검증하기 위해 `axum-highperf-base` 프로젝트를 설계·고도화하며 세 가지 영역에 집중했습니다.

1. **좋은 추상화와 Idiomatic Rust 타입 설계**: 초기 헥사고날 프로토타입에서 `trait ItemRepository`의 제네릭(`<R>`)이 HTTP 핸들러 전역으로 전염되는 문제와 문자열 검증 우회 가능성을 발견했습니다. 이를 **Newtype 패턴(`ItemTitle`, `ItemContent`, `SearchQuery`)**, **`thiserror` 표준 에러 통합**, 그리고 힙 할당 없이 정적 디스패치를 유지하는 **닫힌 `enum ItemRepo` 다형성**으로 리팩토링하여 컴파일 타임에 도메인 불변식을 강제했습니다.
2. **정보 검색 엔진(역색인 + Okapi BM25) 직접 구현과 실측 최적화**: 외부 검색 크레이트에 의존하지 않고 한·영 유니코드 토크나이저, 포스팅 리스트 기반 역색인, 제목 가중치 부스팅을 적용한 Okapi BM25($k_1=1.2, b=0.75$) 랭킹 엔진을 직접 구현했습니다. 10,000건 과학 문서 벤치마크(`examples/bench.rs`)에서 선형 탐색 대비 **QPS 10.8배 향상(583.7 → 6,330.1 QPS)** 및 **p99 지연시간 90.8% 단축(1,853.2 µs → 170.2 µs)**을 실측했습니다. 또한 고동시성 쓰기 락 경합을 64-Shard `RwLock`으로 분산하여 **처리량을 4.43배(3.40M → 15.06M ops/sec)** 끌어올렸습니다.
3. **재현 가능한 빌드와 프로덕션 운영 파이프라인**: `flake.nix` 기반의 재현 가능한 Nix 빌드 환경, 외부 `curl` 없는 자체 TCP 헬스체크(`--healthcheck`)를 포함한 2.0 MB 멀티 스테이지 컨테이너, Kubernetes Restricted PSS 매니페스트, 그리고 GitHub Actions → GHCR → Argo CD 무중단 GitOps 파이프라인을 Arch Linux `kind` 클러스터에서 end-to-end로 검증했습니다.

아스테로모프에서 대규모 과학 문헌 인덱싱·랭킹 파이프라인과 Rust 에이전트 오케스트레이션 백엔드의 성능을 측정 가능한 수치로 끌어올리는 데 직접 기여하고 싶습니다.

---

## 2. `career-ops` `cv.md` 불릿 & `article-digest.md` Proof Points

### `cv.md` 삽입용 프로젝트 불릿

**axum-highperf-base — High-Performance Rust Backend, In-Memory BM25 Search Engine & GitOps Pipeline** (`github.com/jinhosoon/axum-highperf-base`)
- Designed an idiomatic Rust (`Axum 0.8`, `Tokio`, `Tower`) backend combining pragmatic hexagonal boundaries, `thiserror` error modeling, Newtype domain invariants (`ItemTitle`, `ItemContent`, `SearchQuery`), and closed-enum (`ItemRepo`) zero-allocation static dispatch.
- Built a zero-dependency in-memory **Inverted Index + Okapi BM25** ($k_1=1.2, b=0.75$, 2x title boost) search engine (`GET /api/items/search`) indexing **10,000 documents in 92.1 ms (108,523 docs/sec)**, achieving **10.8x higher QPS (6,330.1 vs 583.7 QPS)** and **90.8% lower p99 latency (170.2 µs vs 1,853.2 µs)** over linear scan in reproducible benchmarks (`examples/bench.rs`).
- Eliminated global write-lock contention by replacing a single `RwLock<HashMap>` with a **64-Shard `RwLock`** and lock-free CAS capacity tracking, lifting 32-task mixed R/W throughput **4.43x (3.40M → 15.06M ops/sec)** and sustaining **5,853.1 RPS** HTTP E2E search throughput (`p99 = 180.3 µs`).
- Hardened deployment with a **2.0 MB** multi-stage non-root Docker image (self-contained TCP `--healthcheck`), **Nix Flakes** (`flake.nix`), Kubernetes Restricted PSS + HPA (`metrics-server`), and an automated **GitHub Actions (RustSec audit) → GHCR → Argo CD** GitOps pipeline verified end-to-end on Arch Linux (`kind`).

### `article-digest.md` Proof Points 블록

- **Hero metrics**:
  - Search speedup (10,000 docs): **10.8x QPS** (`583.7` → `6,330.1 QPS`), **p99 latency -90.8%** (`1,853.2 µs` → `170.2 µs`)
  - Indexing speed: **108,523 docs/sec** (`10,000 docs` in `92.1 ms`)
  - Sharded concurrency (32 tasks, 80% R / 20% W): **4.43x throughput** (`3,400,395` → `15,055,618 ops/sec`)
  - HTTP Router E2E: `GET /health` **463,319.2 RPS** (`p99 = 2.3 µs`), `GET /api/items/search` **5,853.1 RPS** (`p99 = 180.3 µs`)
  - Release binary size: **2.0 MB** (`opt-level=3`, `lto=true`, `strip=true`)
- **Architecture**: Pragmatic Hexagonal (Domain Core zero-dependency) + Closed `enum ItemRepo` static dispatch + 64-Shard `RwLock` + Inverted Index / Okapi BM25 + Nix Flakes + K8s Restricted PSS + Argo CD GitOps
- **Key decisions**:
  - Replaced `<R: ItemRepository>` turbofish propagation with closed `enum ItemRepo` while retaining zero heap allocation.
  - Enforced domain invariants at compile time via Newtypes (`ItemTitle`, `ItemContent`, `SearchQuery`) instead of ad-hoc runtime string checks.
  - Built reproducible built-in benchmark suite (`examples/bench.rs`) so every claim is verifiable with a single `cargo run --release --example bench`.

---

## 3. Interview Pack — STAR+R 스토리 4선 (`modes/project.md` 규격)

### Story 1: 제네릭 전염 제거와 타입 레벨 불변식 — RPITIT에서 닫힌 `enum ItemRepo` + Newtype으로의 진화
- **Situation**: 초기 헥사고날 구조에서 `async-trait`의 힙 할당을 피하려 RPITIT `trait ItemRepository`를 도입했으나, 모든 HTTP 핸들러에 `<R: ItemRepository>` 제네릭과 라우터 터보피쉬(`::<R>`)가 전염되고, `Item.title: String` 필드는 생성 이후 검증 우회 위험이 있었습니다.
- **Task**: 런타임 힙 할당 0(Zero-cost)을 유지하면서도 핸들러 시그니처를 간결하게 만들고, 잘못된 도메인 상태 자체가 컴파일되지 않도록 타입 시스템을 활용해야 했습니다.
- **Action**: ① `ItemTitle`, `ItemContent`, `SearchQuery` Newtype 패턴을 도입해 `try_new()`를 통과하지 않으면 객체 생성이 불가능하게 설계했고, ② `thiserror`로 `AppError`에 표준 `std::error::Error`를 구현했으며, ③ 도메인 계약을 유지하되 애플리케이션 내부 배선을 닫힌 `enum ItemRepo` 다형성으로 전환했습니다.
- **Result**: 모든 HTTP 핸들러에서 제네릭과 터보피쉬를 100% 제거하면서도 동적 디스패치(`Box<dyn Future>`) 오버헤드 0을 유지했고, 단위/통합 테스트 9건 및 `clippy -D warnings` 경고 0건을 달성했습니다.
- **Reflection**: 라이브러리처럼 외부에 열린 확장이 필요한 경우가 아니라면, 애플리케이션 내부 저장소 추상화는 `trait + 제네릭`보다 닫힌 `enum`이 가독성과 컴파일 속도 모두에서 우월하다는 교훈을 얻었습니다.

### Story 2: 선형 탐색 `O(N)` 병목을 역색인 + Okapi BM25로 10.8배 개선
- **Situation**: 단순 CRUD 인메모리 저장소에서 키워드 검색을 구현할 때, 전체 문서를 순회하며 부분 문자열을 비교하는 선형 탐색(`O(N)`)은 문서가 10,000건만 되어도 p99 지연시간이 1.85 ms까지 치솟았습니다.
- **Task**: 외부 검색 엔진 의존성 없이 순수 Rust로 관련도 랭킹이 가능한 고속 검색 파이프라인을 구축하고, CRUD 시 인덱스 정합성을 보장해야 했습니다.
- **Action**: 한·영 유니코드 알파벳/숫자 토크나이저와 `term -> (doc_id -> {title_tf, content_tf})` 포스팅 리스트 역색인을 구축했습니다. 문서 길이 총합을 `O(1)`로 증감 관리하여 Okapi BM25($k_1=1.2, b=0.75$, 제목 2배 가중치) 점수를 후보 문서에만 계산하고, 상위 `limit`개 문서만 해당 샤드에서 읽도록 최적화했습니다.
- **Result**: `examples/bench.rs` 10,000건 실측 기준 QPS **10.8배 향상(583.7 → 6,330.1 QPS)**, p99 지연시간 **90.8% 단축(1,853.2 µs → 170.2 µs)**을 달성했고, HTTP E2E 검색도 **5,853.1 RPS**를 기록했습니다.
- **Reflection**: 추후 문서 수가 수백만 건으로 커지면 현재의 단일 `RwLock<InvertedIndex>`가 쓰기 병목이 되므로, 불변 세그먼트(Immutable Segment) + 백그라운드 병합(Lucene/Tantivy 방식)으로 확장하는 다음 단계를 명확히 설계할 수 있게 되었습니다.

### Story 3: 64-Shard `RwLock`으로 고동시성 쓰기 락 경합 4.43배 완화
- **Situation**: 단일 `Arc<RwLock<HashMap<Uuid, Item>>>` 구조에서는 동시 쓰기 요청이 들어올 때마다 전체 맵에 배타 락이 걸려 읽기 요청까지 직렬화되는 경합 병목이 발생했습니다.
- **Task**: 외부 크레이트 없이도 높은 동시성 처리량을 확보하면서, 기존 `MAX_ITEMS` 용량 제한의 원자성을 깨뜨리지 않아야 했습니다.
- **Action**: UUID의 상위/하위 64비트를 XOR 믹싱하여 64개 샤드(`NUM_SHARDS = 64`)로 분산하는 샤딩 `RwLock` 배열을 구현하고, 전역 아이템 개수 제한은 Lock-free `AtomicUsize` CAS 루프로 원자적 예약을 처리했습니다.
- **Result**: 32개 비동기 태스크가 80% 읽기 / 20% 쓰기를 동시 수행하는 벤치마크에서 처리량이 **3.40M → 15.06M ops/sec로 4.43배 향상**되었습니다.
- **Reflection**: 샤드 수를 2의 거듭제곱(64)으로 두어 비트마스크(`& 63`)로 분기 비용을 최소화했고, 단건 CRUD 경합은 샤딩으로 해결되지만 `find_all()` 같은 전체 스캔은 64개 샤드를 순회하므로 API 설계 시 페이지네이션/검색 중심으로 유도해야 함을 확인했습니다.

### Story 4: Zero-Dependency 자체 TCP 헬스체크 & Arch Linux K8s/Argo CD GitOps 검증
- **Situation**: 프로덕션 컨테이너에서 `HEALTHCHECK`를 위해 `curl`이나 셸 도구를 설치하면 공격 표면과 이미지 크기가 커지고, 로컬 `kind` 클러스터에서는 HPA가 CPU 지표를 수집하지 못해 Argo CD가 `Degraded` 상태에 빠지는 문제가 있었습니다.
- **Task**: 최소 권한 보안(K8s Restricted PSS)과 무중단 배포를 유지하면서 로컬 Arch Linux 환경에서 완전한 GitOps 파이프라인을 검증해야 했습니다.
- **Action**: Rust 바이너리 자체에 `--healthcheck` CLI 모드를 내장해 `curl` 없이 TCP 프로브를 수행하도록 구현하고, `debian:bookworm-slim` non-root(`UID 10001`, 2.0 MB 바이너리) 컨테이너와 `flake.nix`를 구성했습니다. 또한 `kind` 클러스터에 `metrics-server`를 배포하고 `--kubelet-insecure-tls` 패치를 적용해 HPA CPU 메트릭을 연동했습니다.
- **Result**: Argo CD Application이 **`Synced` & `Healthy`** 상태에 도달했고, HPA(`cpu: 1%/50%`)와 Pod 2개 무중단 구동 및 CRUD/검색 API 응답을 실제 클러스터에서 검증 완료했습니다.
- **Reflection**: 로컬 쿠버네티스 환경에서도 `metrics-server`와 Restricted PSS까지 프로덕션과 동일하게 재현해 두면, 운영 환경에서 발생할 수 있는 HPA·보안 컨텍스트 이슈를 사전에 차단할 수 있습니다.
