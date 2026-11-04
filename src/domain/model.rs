use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::error::AppError;

/// 인메모리 저장소 및 입력 필드 보안 제한 상수
pub const MAX_ITEMS: usize = 10_000;
pub const MAX_TITLE_LEN: usize = 200;
pub const MAX_CONTENT_LEN: usize = 10_000;

/// 도메인 아이템 엔티티
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
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
        let title = Self::validate_title(&input.title)?;
        Self::validate_content(&input.content)?;

        Ok(Self {
            id: Uuid::new_v4(),
            title,
            content: input.content,
            created_at: Utc::now(),
        })
    }

    /// 도메인 규칙을 검증하며 아이템 필드 부분 수정
    pub fn apply_update(&mut self, input: UpdateItem) -> Result<(), AppError> {
        if let Some(raw_title) = input.title {
            self.title = Self::validate_title(&raw_title)?;
        }
        if let Some(content) = input.content {
            Self::validate_content(&content)?;
            self.content = content;
        }
        Ok(())
    }

    fn validate_title(raw: &str) -> Result<String, AppError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(AppError::BadRequest("Title cannot be empty".to_string()));
        }
        if trimmed.chars().count() > MAX_TITLE_LEN {
            return Err(AppError::BadRequest(format!(
                "Title cannot exceed {} characters",
                MAX_TITLE_LEN
            )));
        }
        Ok(trimmed.to_string())
    }

    fn validate_content(content: &str) -> Result<(), AppError> {
        if content.chars().count() > MAX_CONTENT_LEN {
            return Err(AppError::BadRequest(format!(
                "Content cannot exceed {} characters",
                MAX_CONTENT_LEN
            )));
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
}
