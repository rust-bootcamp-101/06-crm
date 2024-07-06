use std::net::SocketAddr;

use anyhow::Result;

use crm::{
    pb::{crm_client::CrmClient, WelcomeRequestBuilder},
    AppConfig,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load().expect("Failed to load config");
    let addr = config.server.port;
    let addr: SocketAddr = format!("[::1]:{}", addr).parse()?;
    let mut client = CrmClient::connect(format!("http://{addr}")).await?;

    let req = WelcomeRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .interval(98u32)
        .content_ids([1, 2, 3, 4])
        .build()?;

    let res = client.welcome(req).await?.into_inner();
    println!("Welcome response: {:?}", res);
    Ok(())
}
