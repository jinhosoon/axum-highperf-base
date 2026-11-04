use std::future::Future;
use uuid::Uuid;

use crate::domain::{
    error::AppError,
    model::{CreateItem, Item, UpdateItem},
};

/// Outbound Port: 아이템 저장소 인터페이스
///
/// `async-trait`의 힙 할당(Box<dyn Future>) 오버헤드 없이
/// Rust 네이티브 RPITIT(Return Position Impl Trait In Trait)를 사용하여
/// 정적 디스패치(Zero-cost abstraction)를 보장합니다.
pub trait ItemRepository: Clone + Send + Sync + 'static {
    fn find_all(&self) -> impl Future<Output = Result<Vec<Item>, AppError>> + Send;

    fn find_by_id(&self, id: Uuid) -> impl Future<Output = Result<Option<Item>, AppError>> + Send;

    fn create(&self, input: CreateItem) -> impl Future<Output = Result<Item, AppError>> + Send;

    fn update(
        &self,
        id: Uuid,
        input: UpdateItem,
    ) -> impl Future<Output = Result<Item, AppError>> + Send;

    fn delete(&self, id: Uuid) -> impl Future<Output = Result<(), AppError>> + Send;
}
