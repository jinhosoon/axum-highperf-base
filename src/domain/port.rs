use std::future::Future;
use uuid::Uuid;

use crate::domain::{
    error::AppError,
    model::{CreateItem, Item, ScoredItem, SearchQuery, UpdateItem},
};

/// Outbound Port: 아이템 저장소 및 검색 계약
///
/// 도메인 코어가 외부 저장소 구현체에 의존하지 않도록 계약을 정의합니다.
/// 애플리케이션 내부 배선에서는 제네릭(`<R>`) 전염을 없애기 위해
/// `adapters::outbound::ItemRepo` 닫힌 `enum` 다형성으로 정적 디스패치합니다.
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

    fn search(
        &self,
        query: SearchQuery,
    ) -> impl Future<Output = Result<Vec<ScoredItem>, AppError>> + Send;
}
