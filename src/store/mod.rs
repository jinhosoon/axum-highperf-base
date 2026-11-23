pub mod memory;

use uuid::Uuid;

use crate::domain::{
    error::AppError,
    model::{CreateItem, Item, ScoredItem, SearchQuery, UpdateItem},
    port::ItemRepository,
};

pub use memory::InMemoryItemRepository;

/// 닫힌 다형성(Closed Polymorphism) 기반 저장소 열거형
///
/// 제네릭(`<R: ItemRepository>`)과 터보피쉬(`::<R>`) 전염을 제거하면서도
/// `Box<dyn Future>` 동적 할당 없이 컴파일 타임 정적 디스패치를 유지합니다.
#[derive(Debug, Clone)]
pub enum ItemRepo {
    Memory(InMemoryItemRepository),
}

impl ItemRepo {
    pub fn in_memory() -> Self {
        Self::Memory(InMemoryItemRepository::new())
    }

    pub async fn find_all(&self) -> Result<Vec<Item>, AppError> {
        match self {
            Self::Memory(repo) => repo.find_all().await,
        }
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Item>, AppError> {
        match self {
            Self::Memory(repo) => repo.find_by_id(id).await,
        }
    }

    pub async fn create(&self, input: CreateItem) -> Result<Item, AppError> {
        match self {
            Self::Memory(repo) => repo.create(input).await,
        }
    }

    pub async fn update(&self, id: Uuid, input: UpdateItem) -> Result<Item, AppError> {
        match self {
            Self::Memory(repo) => repo.update(id, input).await,
        }
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        match self {
            Self::Memory(repo) => repo.delete(id).await,
        }
    }

    pub async fn search(&self, query: SearchQuery) -> Result<Vec<ScoredItem>, AppError> {
        match self {
            Self::Memory(repo) => repo.search(query).await,
        }
    }
}

impl From<InMemoryItemRepository> for ItemRepo {
    fn from(repo: InMemoryItemRepository) -> Self {
        Self::Memory(repo)
    }
}
