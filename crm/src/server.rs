use std::mem;

use tonic::transport::{Identity, Server, ServerTlsConfig};

use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

use crm::{AppConfig, CrmService};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();
    let mut config = AppConfig::load().expect("Failed to load config");

    // 使用mem::take将tls字段的内存取出来，然后将原来的tls字段置为None，避免使用clone，因为tls我们只在初始化这里用一次
    let tls = mem::take(&mut config.server.tls);
    let addr = config.server.port;
    let addr = format!("[::1]:{}", addr).parse()?;

    info!("CrmService listening on {}", &addr);
    let svc = CrmService::try_new(config).await?.into_server()?;

    let mut server_builder = Server::builder();
    if let Some(tls) = tls {
        let identity = Identity::from_pem(tls.cert, tls.key);
        server_builder = server_builder.tls_config(ServerTlsConfig::new().identity(identity))?;
    }
    server_builder.add_service(svc).serve(addr).await?;
    Ok(())
}
