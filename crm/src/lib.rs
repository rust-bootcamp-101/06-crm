pub mod abi;
mod config;
pub mod pb;

use abi::DecodingKey;
pub use config::AppConfig;

use anyhow::Result;
use tonic::{
    service::interceptor::InterceptedService, transport::Channel, Request, Response, Status,
};

use crm_metadata::pb::metadata_client::MetadataClient;
use crm_send::pb::notification_client::NotificationClient;

use pb::{
    crm_server::{Crm, CrmServer},
    RecallRequest, RecallResponse, RemindRequest, RemindResponse, WelcomeRequest, WelcomeResponse,
};
use user_stat::pb::user_stats_client::UserStatsClient;

#[allow(unused)]
pub struct CrmService {
    config: AppConfig,
    user_stats: UserStatsClient<Channel>,
    notification: NotificationClient<Channel>,
    metadata: MetadataClient<Channel>,
}

#[tonic::async_trait]
impl Crm for CrmService {
    /// user has register X days ago, give them a welcome message
    async fn welcome(
        &self,
        req: Request<WelcomeRequest>,
    ) -> Result<Response<WelcomeResponse>, Status> {
        let req = req.into_inner();
        self.welcome(req).await
    }
    /// last visited or watched in X days, given them something to watch
    async fn recall(
        &self,
        _req: Request<RecallRequest>,
    ) -> Result<Response<RecallResponse>, Status> {
        todo!()
    }
    /// last watched in X days, and user still have unfinished contents
    async fn remind(
        &self,
        _req: Request<RemindRequest>,
    ) -> Result<Response<RemindResponse>, Status> {
        todo!()
    }
}

impl CrmService {
    pub async fn try_new(config: AppConfig) -> Result<Self> {
        let user_stats = UserStatsClient::connect(config.server.user_stats.clone()).await?;
        let notification = NotificationClient::connect(config.server.notification.clone()).await?;
        let metadata = MetadataClient::connect(config.server.metadata.clone()).await?;
        Ok(Self {
            config,
            user_stats,
            notification,
            metadata,
        })
    }

    pub fn into_server(self) -> Result<InterceptedService<CrmServer<Self>, DecodingKey>> {
        let dk = DecodingKey::load(&self.config.auth.pk)?;
        Ok(CrmServer::with_interceptor(self, dk))
    }
}
