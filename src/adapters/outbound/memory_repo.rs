use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::{
    error::AppError,
    model::{CreateItem, Item, UpdateItem, MAX_ITEMS},
    port::ItemRepository,
};

/// Outbound Adapter: 인메모리 해시맵 기반 아이템 저장소 구현체
#[derive(Debug, Clone, Default)]
pub struct InMemoryItemRepository {
    store: Arc<RwLock<HashMap<Uuid, Item>>>,
}

impl InMemoryItemRepository {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ItemRepository for InMemoryItemRepository {
    async fn find_all(&self) -> Result<Vec<Item>, AppError> {
        let store = self.store.read().await;
        Ok(store.values().cloned().collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Item>, AppError> {
        let store = self.store.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn create(&self, input: CreateItem) -> Result<Item, AppError> {
        let item = Item::try_new(input)?;

        let mut store = self.store.write().await;
        if store.len() >= MAX_ITEMS {
            return Err(AppError::TooManyRequests(
                "Maximum storage capacity reached".to_string(),
            ));
        }

        store.insert(item.id, item.clone());
        Ok(item)
    }

    async fn update(&self, id: Uuid, input: UpdateItem) -> Result<Item, AppError> {
        let mut store = self.store.write().await;
        let item = store
            .get_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("Item with id '{}' not found", id)))?;

        item.apply_update(input)?;
        Ok(item.clone())
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let mut store = self.store.write().await;
        store
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| AppError::NotFound(format!("Item with id '{}' not found", id)))
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
    }
}
