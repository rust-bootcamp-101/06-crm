use anyhow::Result;

use crm::pb::{crm_client::CrmClient, WelcomeRequestBuilder};
use tonic::transport::{Certificate, Channel, ClientTlsConfig};
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
    let mut client = CrmClient::new(channel);

    let req = WelcomeRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .interval(98u32)
        .content_ids([1, 2, 3, 4])
        .build()?;

    let res = client.welcome(req).await?.into_inner();
    println!("Welcome response: {:?}", res);
    Ok(())
}
