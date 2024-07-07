use core::fmt;
use std::ops::Deref;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use futures::stream;
use itertools::Itertools;
use prost_types::Timestamp;
use sqlx::PgPool;
use tonic::{Response, Status};
use tracing::info;

use crate::{
    pb::{
        user_stats_server::UserStatsServer, IdQuery, QueryRequest, QueryRequestBuilder,
        RawQueryRequest, TimeQuery, User,
    },
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
        let sql = query.to_string();
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
        println!("query users: {:?}", &ret);
        Ok(Response::new(Box::pin(stream::iter(
            ret.into_iter().map(Ok),
        ))))
    }
}

impl QueryRequest {
    pub fn new_with_date(name: &str, lower: DateTime<Utc>, upper: DateTime<Utc>) -> Self {
        Self::new_with_timestamps(&[(name, lower, upper)])
    }

    pub fn new_with_ids(ids: &[(&str, &[u32])]) -> Self {
        let mut query = QueryRequestBuilder::default();
        for v in ids {
            let iq = IdQuery { ids: v.1.to_vec() };
            query.id((v.0.to_string(), iq));
        }
        query.build().expect("Failed to build query request")
    }

    pub fn new_with_timestamps(timestamps: &[(&str, DateTime<Utc>, DateTime<Utc>)]) -> Self {
        let mut query = QueryRequestBuilder::default();
        for t in timestamps {
            let lower_ts = Timestamp {
                seconds: t.1.timestamp(),
                nanos: 0,
            };
            let upper_ts = Timestamp {
                seconds: t.2.timestamp(),
                nanos: 0,
            };

            let tq = TimeQuery {
                lower: Some(lower_ts),
                upper: Some(upper_ts),
            };
            query.timestamp((t.0.to_string(), tq));
        }

        query.build().expect("Failed to build query request")
    }
}

impl Deref for UserStatsService {
    type Target = UserStatsServiceInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl fmt::Display for QueryRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sql =
            "SELECT name, email, started_but_not_finished FROM user_stats WHERE ".to_string();
        let timestamp_conditions = self
            .timestamps
            .iter()
            .map(|(k, v)| timestamp_query(k, v.lower.as_ref(), v.upper.as_ref()))
            .join(" AND ");

        sql.push_str(&timestamp_conditions);

        let id_conditions = self
            .ids
            .iter()
            .map(|(k, v)| ids_query(k, &v.ids))
            .join(" AND ");

        if !id_conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&id_conditions);
        };

        info!("Generate SQL: {}", sql);

        write!(f, "{}", sql)
    }
}

fn ids_query(name: &str, ids: &[u32]) -> String {
    if ids.is_empty() {
        return "TRUE".to_string();
    }

    // ARRAY{:?} 会被格式化成 ARRAY[1,2,x,..] 这种形势
    // <@ 表示 name 是否包含了 ids
    format!("array{:?} <@ {}", ids, name)
}

fn timestamp_query(name: &str, lower: Option<&Timestamp>, upper: Option<&Timestamp>) -> String {
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

fn ts_to_utc(ts: &Timestamp) -> DateTime<Utc> {
    Utc.timestamp_opt(ts.seconds, ts.nanos as _).unwrap()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use futures::StreamExt;

    use super::*;
    use crate::pb::QueryRequestBuilder;
    use crate::test_utils::{days_to_timestamp, to_idquery, to_timequery};

    #[tokio::test]
    async fn raw_query_should_work() -> Result<()> {
        let (_tdb, svc) = UserStatsService::new_for_test().await?;
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
        let (_tdb, svc) = UserStatsService::new_for_test().await?;
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

        let utc_time = ts_to_utc(&ts).to_rfc3339();
        let query = timestamp_query("created_at", Some(&ts), None);
        assert_eq!(query, format!("created_at >= '{}'", utc_time));

        let query = timestamp_query("created_at", None, Some(&ts));
        assert_eq!(query, format!("created_at <= '{}'", utc_time));

        let end = days_to_timestamp(5);
        let utc_time_end = ts_to_utc(&end).to_rfc3339();
        let query = timestamp_query("created_at", Some(&ts), Some(&end));
        assert_eq!(
            query,
            format!("created_at BETWEEN '{}' AND '{}'", utc_time, utc_time_end)
        );

        Ok(())
    }

    #[test]
    fn query_request_to_string_should_work() -> Result<()> {
        let dt1 = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let dt2 = Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap();
        let query = QueryRequest::new_with_date("created_at", dt1, dt2);
        let sql = query.to_string();
        assert_eq!(sql, "SELECT name, email, started_but_not_finished FROM user_stats WHERE created_at BETWEEN '2024-01-01T00:00:00+00:00' AND '2024-01-02T00:00:00+00:00'");
        Ok(())
    }
}
