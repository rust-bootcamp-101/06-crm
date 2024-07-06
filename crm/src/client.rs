use anyhow::Result;

use crm::pb::{crm_client::CrmClient, WelcomeRequestBuilder};
use tonic::{
    metadata::MetadataValue,
    transport::{Certificate, Channel, ClientTlsConfig},
    Request,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let pem = include_str!("../../fixtures/rootCA.pem");
    let tls = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(pem))
        .domain_name("localhost");
    let channel = Channel::from_static("https://[::1]:50051")
        .tls_config(tls)?
        .connect()
        .await?;

    // 这里作为学习Rust微服务的目的，就不实现jwt签发的功能了，直接生成的token
    let token = include_str!("../../fixtures/token").trim();
    println!("Token: {token}");
    let token: MetadataValue<_> = format!("Bearer {token}").parse()?;

    let mut client = CrmClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    let req = WelcomeRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .interval(98u32)
        .content_ids([1, 2, 3, 4])
        .build()?;

    let res = client.welcome(req).await?.into_inner();
    println!("Welcome response: {:?}", res);
    Ok(())
}
