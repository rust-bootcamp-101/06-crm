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

#[cfg(feature = "test_utils")]
pub mod test_utils {
    use std::{env, path::Path, sync::Arc};

    use anyhow::Result;
    use chrono::{Duration, TimeZone, Utc};
    use prost_types::Timestamp;

    use crate::{
        pb::{IdQuery, TimeQuery},
        AppConfig, UserStatsService, UserStatsServiceInner,
    };

    use sqlx::{Executor, PgPool};
    use sqlx_db_tester::TestPg;

    impl UserStatsService {
        pub async fn new_for_test() -> Result<(TestPg, Self)> {
            let config = AppConfig::load()?;
            let post = config.server.db_url.rfind('/').expect("invalid db_url");
            let server_url = &config.server.db_url[..post];
            let (tdb, pool) = get_test_pool(Some(server_url)).await;
            let svc = Self {
                inner: Arc::new(UserStatsServiceInner { config, pool }),
            };
            Ok((tdb, svc))
        }
    }

    pub async fn get_test_pool(url: Option<&str>) -> (TestPg, PgPool) {
        let url = match url {
            Some(url) => url.to_string(),
            None => "postgres://postgres:postgres@localhost:5432".to_string(),
        };
        let migrations = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("migrations");
        let tdb = TestPg::new(url, migrations);
        let pool = tdb.get_pool().await;

        // run prepared sql to insert test data
        let sql = include_str!("../fixtures/test.sql").split(';');
        let mut ts = pool.begin().await.expect("begin transaction failed");
        for s in sql {
            if s.trim().is_empty() {
                continue;
            }
            ts.execute(s).await.expect("execute sql failed");
        }
        ts.commit().await.expect("commit transaction failed");

        // 注意: 此tdb一定要返回出去，即使外面不使用，也要接收 成 _tdb，因为在外部的scope中，tdb用来作为生命周期约束，drop掉测试数据
        (tdb, pool)
    }

    pub fn to_idquery(id: &[u32]) -> IdQuery {
        IdQuery { ids: id.to_vec() }
    }

    pub fn to_timequery(lower: Option<i64>, upper: Option<i64>) -> TimeQuery {
        TimeQuery {
            lower: lower.map(days_to_timestamp),
            upper: upper.map(days_to_timestamp),
        }
    }

    pub fn days_to_timestamp(days: i64) -> Timestamp {
        let dt = Utc
            .with_ymd_and_hms(2024, 7, 6, 0, 0, 0)
            .unwrap()
            .checked_sub_signed(Duration::days(days))
            .unwrap();
        Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }
    }
}
