use anyhow::Result;

use chrono::{TimeZone, Utc};
use crm::pb::{
    crm_client::CrmClient, RecallRequestBuilder, RemindRequestBuilder, WelcomeRequestBuilder,
};
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
    let token: MetadataValue<_> = format!("Bearer {token}").parse()?;

    let mut client = CrmClient::with_interceptor(channel, move |mut req: Request<()>| {
        // 每一次请求都会执行这个闭包，所以token需要clone
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    // 欢迎用户
    let since = Utc.with_ymd_and_hms(2024, 3, 31, 0, 0, 0).unwrap();
    let interval = Utc::now().signed_duration_since(since).num_days();
    let req = WelcomeRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .interval(interval as u32)
        .content_ids([1, 2, 3, 4])
        .build()?;

    let res = client.welcome(req).await?.into_inner();
    println!("Welcome response: {:?}", res);

    // 用户在x天之前浏览过/观看过，给他们一些推荐内容
    let since = Utc.with_ymd_and_hms(2024, 5, 30, 0, 0, 0).unwrap();
    let visit_interval = Utc::now().signed_duration_since(since).num_days();
    let since = Utc.with_ymd_and_hms(2024, 6, 28, 0, 0, 0).unwrap();
    let watched_interval = Utc::now().signed_duration_since(since).num_days();
    let req = RecallRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .last_visit_interval(visit_interval as u32)
        .last_watched_interval(watched_interval as u32)
        .content_ids([1, 2, 3, 4])
        .build()?;

    let res = client.recall(req).await?.into_inner();
    println!("Recall response: {:?}", res);

    // 用户在x天之前观看过、但未看完，推送消息告诉他，您有未观看完的视频
    let since = Utc.with_ymd_and_hms(2024, 5, 30, 0, 0, 0).unwrap();
    let visit_interval = Utc::now().signed_duration_since(since).num_days();
    let req = RemindRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .last_visit_interval(visit_interval as u32)
        .build()?;

    let res = client.remind(req).await?.into_inner();
    println!("Remind response: {:?}", res);
    Ok(())
}
