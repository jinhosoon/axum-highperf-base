use std::fmt;
use std::ops::Deref;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::error::AppError;

/// 인메모리 저장소 및 입력 필드 보안 제한 상수
pub const MAX_ITEMS: usize = 50_000;
pub const MAX_TITLE_LEN: usize = 200;
pub const MAX_CONTENT_LEN: usize = 10_000;
pub const MAX_QUERY_LEN: usize = 200;
pub const DEFAULT_SEARCH_LIMIT: usize = 10;
pub const MAX_SEARCH_LIMIT: usize = 100;

/// 검증된 아이템 제목 Newtype ("Parse, don't validate")
///
/// 오직 `ItemTitle::try_new`를 통과해야만 생성될 수 있어
/// 잘못된(빈 문자열 또는 길이 초과) 제목 상태 자체가 컴파일 타임에 불가능합니다.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemTitle(String);

impl ItemTitle {
    pub fn try_new(raw: impl AsRef<str>) -> Result<Self, AppError> {
        let trimmed = raw.as_ref().trim();
        if trimmed.is_empty() {
            return Err(AppError::BadRequest("Title cannot be empty".to_string()));
        }
        if trimmed.chars().count() > MAX_TITLE_LEN {
            return Err(AppError::BadRequest(format!(
                "Title cannot exceed {} characters",
                MAX_TITLE_LEN
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for ItemTitle {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for ItemTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ItemTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<&str> for ItemTitle {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// 검증된 아이템 본문 Newtype ("Parse, don't validate")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemContent(String);

impl ItemContent {
    pub fn try_new(raw: impl Into<String>) -> Result<Self, AppError> {
        let content = raw.into();
        if content.chars().count() > MAX_CONTENT_LEN {
            return Err(AppError::BadRequest(format!(
                "Content cannot exceed {} characters",
                MAX_CONTENT_LEN
            )));
        }
        Ok(Self(content))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for ItemContent {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for ItemContent {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ItemContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<&str> for ItemContent {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// 검증된 검색 질의 도메인 모델
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    query: String,
    limit: usize,
}

impl SearchQuery {
    pub fn try_new(raw_query: impl AsRef<str>, limit: Option<usize>) -> Result<Self, AppError> {
        let trimmed = raw_query.as_ref().trim();
        if trimmed.is_empty() {
            return Err(AppError::BadRequest(
                "Search query 'q' cannot be empty".to_string(),
            ));
        }
        if trimmed.chars().count() > MAX_QUERY_LEN {
            return Err(AppError::BadRequest(format!(
                "Search query cannot exceed {} characters",
                MAX_QUERY_LEN
            )));
        }
        let resolved_limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
        if resolved_limit == 0 || resolved_limit > MAX_SEARCH_LIMIT {
            return Err(AppError::BadRequest(format!(
                "Search limit must be between 1 and {}",
                MAX_SEARCH_LIMIT
            )));
        }
        Ok(Self {
            query: trimmed.to_string(),
            limit: resolved_limit,
        })
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
}

/// 도메인 아이템 엔티티
///
/// 필드가 `pub`이어도 `ItemTitle` / `ItemContent` Newtype이
/// 도메인 불변식을 타입 레벨에서 강제합니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: Uuid,
    pub title: ItemTitle,
    pub content: ItemContent,
    pub created_at: DateTime<Utc>,
}

/// BM25 관련도 점수가 포함된 검색 결과 모델
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredItem {
    pub item: Item,
    pub score: f64,
}

/// 아이템 생성 요청 모델
#[derive(Debug, Deserialize)]
pub struct CreateItem {
    pub title: String,
    pub content: String,
}

/// 아이템 수정 요청 모델 (부분 업데이트)
#[derive(Debug, Deserialize)]
pub struct UpdateItem {
    pub title: Option<String>,
    pub content: Option<String>,
}

impl Item {
    /// 도메인 불변식(길이 및 공백 규칙)을 검증하여 새 아이템 생성
    pub fn try_new(input: CreateItem) -> Result<Self, AppError> {
        let title = ItemTitle::try_new(&input.title)?;
        let content = ItemContent::try_new(input.content)?;

        Ok(Self {
            id: Uuid::new_v4(),
            title,
            content,
            created_at: Utc::now(),
        })
    }

    /// 도메인 규칙을 검증하며 아이템 필드 부분 수정
    pub fn apply_update(&mut self, input: UpdateItem) -> Result<(), AppError> {
        if let Some(raw_title) = input.title {
            self.title = ItemTitle::try_new(&raw_title)?;
        }
        if let Some(raw_content) = input.content {
            self.content = ItemContent::try_new(raw_content)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_valid_item_and_trims_title() {
        let item = Item::try_new(CreateItem {
            title: "  Hello Rust  ".to_string(),
            content: "Hexagonal Architecture".to_string(),
        })
        .unwrap();

        assert_eq!(item.title, "Hello Rust");
        assert_eq!(item.content, "Hexagonal Architecture");
    }

    #[test]
    fn rejects_empty_title() {
        let res = Item::try_new(CreateItem {
            title: "   ".to_string(),
            content: "body".to_string(),
        });
        assert!(matches!(res, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn rejects_oversized_title() {
        let res = Item::try_new(CreateItem {
            title: "a".repeat(MAX_TITLE_LEN + 1),
            content: "body".to_string(),
        });
        assert!(matches!(res, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn validates_search_query_and_limit() {
        let q = SearchQuery::try_new("  rust bm25  ", None).unwrap();
        assert_eq!(q.query(), "rust bm25");
        assert_eq!(q.limit(), DEFAULT_SEARCH_LIMIT);

        assert!(matches!(
            SearchQuery::try_new("   ", Some(10)),
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            SearchQuery::try_new("rust", Some(0)),
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            SearchQuery::try_new("rust", Some(MAX_SEARCH_LIMIT + 1)),
            Err(AppError::BadRequest(_))
        ));
    }
}
