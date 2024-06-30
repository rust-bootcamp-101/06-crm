mod abi;
mod config;
pub mod pb;

use std::pin::Pin;

use futures::Stream;
use tonic::{Request, Response, Status, Streaming};

pub use config::AppConfig;
use pb::{
    metadata_server::{Metadata, MetadataServer},
    Content, MaterializeRequest,
};

#[allow(unused)]
pub struct MetadataService {
    config: AppConfig,
}

pub type ServiceResult<T> = Result<Response<T>, Status>;
pub type ResponseStream = Pin<Box<dyn Stream<Item = Result<Content, Status>> + Send>>;

#[tonic::async_trait]
impl Metadata for MetadataService {
    /// Server streaming response type for the Materialize method.
    type MaterializeStream = ResponseStream;

    async fn materialize(
        &self,
        request: Request<Streaming<MaterializeRequest>>,
    ) -> ServiceResult<Self::MaterializeStream> {
        let query = request.into_inner();
        self.materialize(query).await
    }
}

impl MetadataService {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn into_server(self) -> MetadataServer<Self> {
        MetadataServer::new(self)
    }
}
