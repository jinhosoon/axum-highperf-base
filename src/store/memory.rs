use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::{
    error::AppError,
    model::{CreateItem, Item, ScoredItem, SearchQuery, UpdateItem, MAX_ITEMS},
    port::ItemRepository,
};

/// 동시성 락 경합 완화를 위한 샤드 개수 (2의 거듭제곱)
pub const NUM_SHARDS: usize = 64;

/// Okapi BM25 하이퍼파라미터 및 필드 가중치
const BM25_K1: f64 = 1.2;
const BM25_B: f64 = 0.75;
const TITLE_WEIGHT: f64 = 2.0;
const CONTENT_WEIGHT: f64 = 1.0;

#[derive(Debug, Default, Clone, Copy)]
struct DocTermStats {
    title_tf: u32,
    content_tf: u32,
}

#[derive(Debug, Default, Clone, Copy)]
struct DocLength {
    title_len: u32,
    content_len: u32,
}

/// 인메모리 역색인(Inverted Index) + Okapi BM25 랭킹 인덱스
#[derive(Debug, Default)]
struct InvertedIndex {
    /// term -> (doc_id -> term frequency in title/content)
    postings: HashMap<String, HashMap<Uuid, DocTermStats>>,
    /// doc_id -> field token counts
    doc_lengths: HashMap<Uuid, DocLength>,
    total_title_len: u64,
    total_content_len: u64,
}

impl InvertedIndex {
    fn index_item(&mut self, item: &Item) {
        let title_tokens = tokenize(item.title.as_str());
        let content_tokens = tokenize(item.content.as_str());

        let title_len = title_tokens.len() as u32;
        let content_len = content_tokens.len() as u32;

        self.doc_lengths.insert(
            item.id,
            DocLength {
                title_len,
                content_len,
            },
        );
        self.total_title_len += title_len as u64;
        self.total_content_len += content_len as u64;

        let mut term_counts: HashMap<String, DocTermStats> = HashMap::new();
        for token in title_tokens {
            term_counts.entry(token).or_default().title_tf += 1;
        }
        for token in content_tokens {
            term_counts.entry(token).or_default().content_tf += 1;
        }

        for (term, stats) in term_counts {
            self.postings
                .entry(term)
                .or_default()
                .insert(item.id, stats);
        }
    }

    fn remove_item(&mut self, item: &Item) {
        if let Some(len) = self.doc_lengths.remove(&item.id) {
            self.total_title_len = self.total_title_len.saturating_sub(len.title_len as u64);
            self.total_content_len = self
                .total_content_len
                .saturating_sub(len.content_len as u64);
        }

        let mut unique_terms: HashSet<String> = HashSet::new();
        for token in tokenize(item.title.as_str()) {
            unique_terms.insert(token);
        }
        for token in tokenize(item.content.as_str()) {
            unique_terms.insert(token);
        }

        for term in unique_terms {
            if let Some(posting) = self.postings.get_mut(&term) {
                posting.remove(&item.id);
                if posting.is_empty() {
                    self.postings.remove(&term);
                }
            }
        }
    }

    /// Okapi BM25 알고리즘으로 후보 문서만 스코어링하여 상위 `limit`개 `(Uuid, score)` 반환
    fn search(&self, query: &SearchQuery) -> Vec<(Uuid, f64)> {
        let total_docs = self.doc_lengths.len() as f64;
        if total_docs == 0.0 {
            return Vec::new();
        }

        let query_terms: HashSet<String> = tokenize(query.query()).into_iter().collect();
        if query_terms.is_empty() {
            return Vec::new();
        }

        let avgdl = ((self.total_title_len as f64 * TITLE_WEIGHT)
            + (self.total_content_len as f64 * CONTENT_WEIGHT))
            / total_docs;
        let safe_avgdl = if avgdl > 0.0 { avgdl } else { 1.0 };

        let mut scores: HashMap<Uuid, f64> = HashMap::new();

        for term in &query_terms {
            let Some(posting) = self.postings.get(term) else {
                continue;
            };
            let df = posting.len() as f64;
            // Lucene/Okapi smoothed IDF (항상 양수 보장)
            let idf = ((total_docs - df + 0.5) / (df + 0.5) + 1.0).ln();

            for (&doc_id, stats) in posting {
                let Some(doc_len) = self.doc_lengths.get(&doc_id) else {
                    continue;
                };
                let tf = (stats.title_tf as f64 * TITLE_WEIGHT)
                    + (stats.content_tf as f64 * CONTENT_WEIGHT);
                let dl = (doc_len.title_len as f64 * TITLE_WEIGHT)
                    + (doc_len.content_len as f64 * CONTENT_WEIGHT);

                let numerator = tf * (BM25_K1 + 1.0);
                let denominator = tf + BM25_K1 * (1.0 - BM25_B + BM25_B * (dl / safe_avgdl));
                let term_score = idf * (numerator / denominator);

                *scores.entry(doc_id).or_insert(0.0) += term_score;
            }
        }

        let mut ranked: Vec<(Uuid, f64)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        ranked.truncate(query.limit());
        ranked
    }
}

/// 한·영 유니코드 알파벳/숫자 기반 토크나이저 (소문자 정규화 + 구두점 분리)
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

#[inline]
fn shard_index(id: Uuid) -> usize {
    let bytes = id.as_bytes();
    let mixed = u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]) ^ u64::from_le_bytes([
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]);
    (mixed as usize) & (NUM_SHARDS - 1)
}

/// Outbound Adapter: 64-Shard `RwLock` + 인메모리 역색인(BM25) 기반 고동시성 저장소
#[derive(Debug, Clone)]
pub struct InMemoryItemRepository {
    shards: Arc<[RwLock<HashMap<Uuid, Item>>; NUM_SHARDS]>,
    index: Arc<RwLock<InvertedIndex>>,
    count: Arc<AtomicUsize>,
}

impl Default for InMemoryItemRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryItemRepository {
    pub fn new() -> Self {
        let shards = std::array::from_fn(|_| RwLock::new(HashMap::new()));
        Self {
            shards: Arc::new(shards),
            index: Arc::new(RwLock::new(InvertedIndex::default())),
            count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ItemRepository for InMemoryItemRepository {
    async fn find_all(&self) -> Result<Vec<Item>, AppError> {
        let mut items = Vec::with_capacity(self.len());
        for shard in self.shards.iter() {
            let guard = shard.read().await;
            items.extend(guard.values().cloned());
        }
        Ok(items)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Item>, AppError> {
        let guard = self.shards[shard_index(id)].read().await;
        Ok(guard.get(&id).cloned())
    }

    async fn create(&self, input: CreateItem) -> Result<Item, AppError> {
        let item = Item::try_new(input)?;

        // Lock-free CAS로 최대 저장 용량 원자적 예약
        let mut current = self.count.load(Ordering::Acquire);
        loop {
            if current >= MAX_ITEMS {
                return Err(AppError::TooManyRequests(
                    "Maximum storage capacity reached".to_string(),
                ));
            }
            match self.count.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }

        {
            let mut shard = self.shards[shard_index(item.id)].write().await;
            shard.insert(item.id, item.clone());
        }
        {
            let mut idx = self.index.write().await;
            idx.index_item(&item);
        }

        Ok(item)
    }

    async fn update(&self, id: Uuid, input: UpdateItem) -> Result<Item, AppError> {
        let (old_item, updated_item) = {
            let mut shard = self.shards[shard_index(id)].write().await;
            let item = shard
                .get_mut(&id)
                .ok_or_else(|| AppError::NotFound(format!("Item with id '{}' not found", id)))?;
            let old = item.clone();
            item.apply_update(input)?;
            (old, item.clone())
        };

        {
            let mut idx = self.index.write().await;
            idx.remove_item(&old_item);
            idx.index_item(&updated_item);
        }

        Ok(updated_item)
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let removed = {
            let mut shard = self.shards[shard_index(id)].write().await;
            shard
                .remove(&id)
                .ok_or_else(|| AppError::NotFound(format!("Item with id '{}' not found", id)))?
        };

        self.count.fetch_sub(1, Ordering::AcqRel);

        {
            let mut idx = self.index.write().await;
            idx.remove_item(&removed);
        }

        Ok(())
    }

    async fn search(&self, query: SearchQuery) -> Result<Vec<ScoredItem>, AppError> {
        let ranked = {
            let idx = self.index.read().await;
            idx.search(&query)
        };

        let mut results = Vec::with_capacity(ranked.len());
        for (doc_id, score) in ranked {
            let guard = self.shards[shard_index(doc_id)].read().await;
            if let Some(item) = guard.get(&doc_id) {
                results.push(ScoredItem {
                    item: item.clone(),
                    score,
                });
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn crud_lifecycle_works() {
        let repo = InMemoryItemRepository::new();

        let created = repo
            .create(CreateItem {
                title: "Item 1".to_string(),
                content: "Content 1".to_string(),
            })
            .await
            .unwrap();

        let fetched = repo.find_by_id(created.id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Item 1");

        let updated = repo
            .update(
                created.id,
                UpdateItem {
                    title: Some("Updated".to_string()),
                    content: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.title, "Updated");

        repo.delete(created.id).await.unwrap();
        assert!(repo.find_by_id(created.id).await.unwrap().is_none());
        assert_eq!(repo.len(), 0);
    }

    #[tokio::test]
    async fn inverted_index_bm25_ranks_title_and_korean_tokens() {
        let repo = InMemoryItemRepository::new();

        let doc_title_match = repo
            .create(CreateItem {
                title: "Rust 역색인 BM25 검색 엔진".to_string(),
                content: "고성능 비동기 백엔드 아키텍처 설계".to_string(),
            })
            .await
            .unwrap();

        let doc_body_match = repo
            .create(CreateItem {
                title: "일반 백엔드 노트".to_string(),
                content: "Rust 언어로 간단한 서버를 만들고 bm25 개념을 참고함".to_string(),
            })
            .await
            .unwrap();

        let _doc_unrelated = repo
            .create(CreateItem {
                title: "Kubernetes Argo CD".to_string(),
                content: "GitOps 무중단 배포 파이프라인".to_string(),
            })
            .await
            .unwrap();

        let q = SearchQuery::try_new("rust bm25", Some(5)).unwrap();
        let results = repo.search(q).await.unwrap();

        assert_eq!(results.len(), 2);
        // 제목 가중치 부스팅 덕분에 doc_title_match가 1순위여야 함
        assert_eq!(results[0].item.id, doc_title_match.id);
        assert_eq!(results[1].item.id, doc_body_match.id);
        assert!(results[0].score > results[1].score);

        // 한글 토큰 검색 및 삭제 후 인덱스 동기화 검증
        let kr_results = repo
            .search(SearchQuery::try_new("역색인", Some(5)).unwrap())
            .await
            .unwrap();
        assert_eq!(kr_results.len(), 1);
        assert_eq!(kr_results[0].item.id, doc_title_match.id);

        repo.delete(doc_title_match.id).await.unwrap();
        let after_delete = repo
            .search(SearchQuery::try_new("역색인", Some(5)).unwrap())
            .await
            .unwrap();
        assert!(after_delete.is_empty());
    }
}
