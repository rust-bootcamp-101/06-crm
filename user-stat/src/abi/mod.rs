use std::ops::Deref;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use futures::stream;
use itertools::Itertools;
use prost_types::Timestamp;
use sqlx::PgPool;
use tonic::{Response, Status};

use crate::{
    pb::{user_stats_server::UserStatsServer, QueryRequest, RawQueryRequest, User},
    AppConfig, ResponseStream, ServiceResult, UserStatsService, UserStatsServiceInner,
};

impl UserStatsService {
    pub async fn new(config: AppConfig) -> Self {
        let pool = PgPool::connect(&config.server.db_url)
            .await
            .expect("Failed to connect to db");
        let inner = UserStatsServiceInner { config, pool };
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn into_server(self) -> UserStatsServer<UserStatsService> {
        UserStatsServer::new(self)
    }
    pub async fn query(&self, query: QueryRequest) -> ServiceResult<ResponseStream> {
        // generate sql base on query
        let mut sql = "SELECT name, email FROM user_stats WHERE ".to_string();
        let timestamp_conditions = query
            .timestamps
            .into_iter()
            .map(|(k, v)| timestamp_query(&k, v.lower, v.upper))
            .join(" AND ");

        sql.push_str(&timestamp_conditions);

        let id_conditions = query
            .ids
            .into_iter()
            .map(|(k, v)| ids_query(&k, v.ids))
            .join(" AND ");

        if !id_conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&id_conditions);
        }

        self.raw_query(RawQueryRequest { query: sql }).await
    }

    pub async fn raw_query(&self, req: RawQueryRequest) -> ServiceResult<ResponseStream> {
        // TODO: query must only email and name, so we should use sqlparser to parse the query
        let Ok(ret) = sqlx::query_as::<_, User>(&req.query)
            .fetch_all(&self.pool)
            .await
        else {
            return Err(Status::internal(format!(
                "Failed to fetch data with query {}",
                req.query
            )));
        };

        Ok(Response::new(Box::pin(stream::iter(
            ret.into_iter().map(Ok),
        ))))
    }
}

impl Deref for UserStatsService {
    type Target = UserStatsServiceInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

fn ids_query(name: &str, ids: Vec<u32>) -> String {
    if ids.is_empty() {
        return "TRUE".to_string();
    }

    // ARRAY{:?} 会被格式化成 ARRAY[1,2,x,..] 这种形势
    // <@ 表示 name 是否包含了 ids
    format!("array{:?} <@ {}", ids, name)
}

fn timestamp_query(name: &str, lower: Option<Timestamp>, upper: Option<Timestamp>) -> String {
    match (lower, upper) {
        (None, None) => "".to_string(),
        (Some(lower), None) => {
            let lower = ts_to_utc(lower);
            format!("{} >= '{}'", name, lower.to_rfc3339())
        }
        (None, Some(upper)) => {
            let upper = ts_to_utc(upper);
            format!("{} <= '{}'", name, upper.to_rfc3339())
        }
        (Some(lower), Some(upper)) => {
            let lower = ts_to_utc(lower);
            let upper = ts_to_utc(upper);
            format!(
                "{} BETWEEN '{}' AND '{}'",
                name,
                lower.to_rfc3339(),
                upper.to_rfc3339()
            )
        }
    }
}

fn ts_to_utc(ts: Timestamp) -> DateTime<Utc> {
    Utc.timestamp_opt(ts.seconds, ts.nanos as _).unwrap()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chrono::{Duration, Utc};
    use futures::StreamExt;

    use super::*;
    use crate::pb::{IdQuery, QueryRequestBuilder, TimeQuery};
    use crate::AppConfig;

    #[tokio::test]
    async fn raw_query_should_work() -> Result<()> {
        let config = AppConfig::load().expect("Failed to load config");
        let svc = UserStatsService::new(config).await;
        let mut stream = svc
            .raw_query(RawQueryRequest {
                query: "SELECT * FROM user_stats WHERE created_at > '2024-01-01' LIMIT 5"
                    .to_string(),
            })
            .await?
            .into_inner();
        while let Some(res) = stream.next().await {
            println!("{:?}", res);
        }

        Ok(())
    }

    #[tokio::test]
    async fn query_should_work() -> Result<()> {
        let config = AppConfig::load().expect("Failed to load config");
        let svc = UserStatsService::new(config).await;
        let query = QueryRequestBuilder::default()
            .timestamp(("created_at".to_string(), to_timequery(Some(220), None)))
            .timestamp(("last_visited_at".to_string(), to_timequery(Some(50), None)))
            .id(("viewed_but_not_started".to_string(), to_idquery(&[269904])))
            .build()?;

        let mut stream = svc.query(query).await?.into_inner();
        while let Some(res) = stream.next().await {
            println!("{:?}", res);
        }
        Ok(())
    }

    #[test]
    fn timestamp_query_should_work() -> Result<()> {
        let query = timestamp_query("created_at", None, None);
        assert!(query.is_empty());
        let ts = days_to_timestamp(15);

        let utc_time = ts_to_utc(ts.clone()).to_rfc3339();
        let query = timestamp_query("created_at", Some(ts.clone()), None);
        assert_eq!(query, format!("created_at >= '{}'", utc_time));

        let query = timestamp_query("created_at", None, Some(ts.clone()));
        assert_eq!(query, format!("created_at <= '{}'", utc_time));

        let end = days_to_timestamp(5);
        let utc_time_end = ts_to_utc(end.clone()).to_rfc3339();
        let query = timestamp_query("created_at", Some(ts), Some(end));
        assert_eq!(
            query,
            format!("created_at BETWEEN '{}' AND '{}'", utc_time, utc_time_end)
        );

        Ok(())
    }

    fn to_idquery(id: &[u32]) -> IdQuery {
        IdQuery { ids: id.to_vec() }
    }

    fn to_timequery(lower: Option<i64>, upper: Option<i64>) -> TimeQuery {
        TimeQuery {
            lower: lower.map(days_to_timestamp),
            upper: upper.map(days_to_timestamp),
        }
    }

    fn days_to_timestamp(days: i64) -> Timestamp {
        let dt = Utc::now().checked_sub_signed(Duration::days(days)).unwrap();
        Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }
    }
}
