use std::{net::SocketAddr, time::Duration};

use anyhow::Result;
use crm_metadata::{
    pb::{metadata_client::MetadataClient, MaterializeRequest},
    AppConfig, MetadataService,
};
use futures::StreamExt;
use tokio::time::sleep;
use tonic::transport::Server;

#[tokio::test]
async fn materialize_should_work() -> Result<()> {
    let addr = start_server().await?;
    let req = tokio_stream::iter(vec![
        MaterializeRequest { id: 1 },
        MaterializeRequest { id: 2 },
        MaterializeRequest { id: 3 },
    ]);

    let mut client = MetadataClient::connect(format!("http://{addr}")).await?;
    let stream = client.materialize(req).await?.into_inner();
    let ret = stream
        .then(|res| async move { res.unwrap() })
        .collect::<Vec<_>>()
        .await;
    assert_eq!(ret.len(), 3);
    Ok(())
}

async fn start_server() -> Result<SocketAddr> {
    let config = AppConfig::load()?;
    let addr = config.server.port + 10; // 避免测试端口冲突
    let addr = format!("[::1]:{}", addr).parse()?;

    let svc = MetadataService::new(config).into_server();
    tokio::spawn(async move {
        Server::builder()
            .add_service(svc)
            .serve(addr)
            .await
            .unwrap();
    });
    // 等待服务启动
    sleep(Duration::from_millis(10)).await;
    Ok(addr)
}
