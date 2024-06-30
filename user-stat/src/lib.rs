mod abi;
mod config;
pub mod pb;

use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use sqlx::PgPool;
use tonic::{Request, Response, Status};

pub use config::AppConfig;
use pb::{user_stats_server::UserStats, QueryRequest, RawQueryRequest, User};

#[derive(Clone)]
pub struct UserStatsService {
    inner: Arc<UserStatsServiceInner>,
}

#[allow(unused)]
pub struct UserStatsServiceInner {
    config: AppConfig,
    pool: PgPool,
}

pub type ServiceResult<T> = Result<Response<T>, Status>;
pub type ResponseStream = Pin<Box<dyn Stream<Item = Result<User, Status>> + Send>>;

#[tonic::async_trait]
impl UserStats for UserStatsService {
    type QueryStream = ResponseStream;
    type RawQueryStream = ResponseStream;

    async fn query(&self, request: Request<QueryRequest>) -> ServiceResult<Self::QueryStream> {
        let req = request.into_inner();
        self.query(req).await
    }

    async fn raw_query(
        &self,
        request: Request<RawQueryRequest>,
    ) -> ServiceResult<Self::RawQueryStream> {
        let req = request.into_inner();
        self.raw_query(req).await
    }
}
