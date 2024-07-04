use std::{net::SocketAddr, time::Duration};

use anyhow::Result;
use futures::StreamExt;
use sqlx_db_tester::TestPg;
use tokio::time::sleep;
use tonic::transport::Server;
use user_stat::{
    pb::{user_stats_client::UserStatsClient, QueryRequestBuilder, RawQueryRequestBuilder},
    test_utils::{to_idquery, to_timequery},
    AppConfig, UserStatsService,
};

#[tokio::test]
async fn raw_query_should_work() -> Result<()> {
    let (_tdb, addr) = start_server(100).await?;
    let req = RawQueryRequestBuilder::default()
        .query("SELECT * FROM user_stats WHERE created_at > '2024-01-01' LIMIT 5")
        .build()?;
    let mut client = UserStatsClient::connect(format!("http://{addr}")).await?;
    let stream = client.raw_query(req).await?.into_inner();
    let users = stream
        .then(|res| async move { res.unwrap() })
        .collect::<Vec<_>>()
        .await;
    assert_eq!(users.len(), 5);
    Ok(())
}

#[tokio::test]
async fn query_should_work() -> Result<()> {
    let (_tdb, addr) = start_server(200).await?;
    let req = QueryRequestBuilder::default()
        .timestamp(("created_at".to_string(), to_timequery(Some(220), None)))
        .timestamp(("last_visited_at".to_string(), to_timequery(Some(50), None)))
        .id(("viewed_but_not_started".to_string(), to_idquery(&[269904])))
        .build()?;

    let mut client = UserStatsClient::connect(format!("http://{addr}")).await?;
    let stream = client.query(req).await?.into_inner();
    let users = stream
        .then(|res| async move { res.unwrap() })
        .collect::<Vec<_>>()
        .await;
    assert_eq!(users.len(), 0);
    Ok(())
}

async fn start_server(port: u16) -> Result<(TestPg, SocketAddr)> {
    let config = AppConfig::load()?;
    let addr = config.server.port + port; // 避免测试端口冲突，此方法不是特别号
    let addr = format!("[::1]:{}", addr).parse()?;

    let (tdb, svc) = UserStatsService::new_for_test().await?;
    tokio::spawn(async move {
        Server::builder()
            .add_service(svc.into_server())
            .serve(addr)
            .await
            .unwrap();
    });
    // 等待服务启动
    sleep(Duration::from_millis(10)).await;
    Ok((tdb, addr))
}
