mod abi;
mod config;
pub mod pb;

use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, Streaming};

pub use config::AppConfig;
use pb::{notification_server::Notification, send_request::Msg, SendRequest, SendResponse};

#[derive(Clone)]
pub struct NotificationService {
    inner: Arc<NotificationServiceInner>,
}

#[allow(unused)]
pub struct NotificationServiceInner {
    config: AppConfig,
    sender: mpsc::Sender<Msg>,
}

pub type ServiceResult<T> = Result<Response<T>, Status>;
pub type ResponseStream = Pin<Box<dyn Stream<Item = Result<SendResponse, Status>> + Send>>;

#[tonic::async_trait]
impl Notification for NotificationService {
    /// Server streaming response type for the Materialize method.
    type SendStream = ResponseStream;

    async fn send(
        &self,
        request: Request<Streaming<SendRequest>>,
    ) -> ServiceResult<Self::SendStream> {
        let query = request.into_inner();
        self.send(query).await
    }
}
