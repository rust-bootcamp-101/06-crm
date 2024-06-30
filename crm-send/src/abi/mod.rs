mod email;
mod in_app;
mod sms;

use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use futures::Stream;
use futures::StreamExt;
use prost_types::Timestamp;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::{info, warn};

use crate::{
    pb::{
        notification_server::NotificationServer, send_request::Msg, EmailMessage, InAppMessage,
        SendRequest, SendResponse, SmsMessage,
    },
    AppConfig, NotificationService, NotificationServiceInner, ResponseStream, ServiceResult,
};

pub trait Sender {
    async fn send(self, svc: NotificationService) -> Result<SendResponse, Status>;
}

const CHANNEL_SIZE: usize = 1024;

impl NotificationService {
    pub fn new(config: AppConfig) -> Self {
        let sender = dummy_send();
        let inner = NotificationServiceInner { config, sender };
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn into_server(self) -> NotificationServer<Self> {
        NotificationServer::new(self)
    }

    pub async fn send(
        &self,
        mut stream: impl Stream<Item = Result<SendRequest, Status>> + Send + 'static + Unpin,
    ) -> ServiceResult<ResponseStream> {
        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
        let notifi = self.clone();
        tokio::spawn(async move {
            while let Some(Ok(req)) = stream.next().await {
                let notifi_clone = notifi.clone();
                let res = match req.msg {
                    Some(Msg::Email(email)) => email.send(notifi_clone).await,
                    Some(Msg::Sms(sms)) => sms.send(notifi_clone).await,
                    Some(Msg::InApp(in_app)) => in_app.send(notifi_clone).await,
                    None => {
                        warn!("Invalid request");
                        Err(Status::invalid_argument("msg is required"))
                    }
                };
                let _ = tx.send(res).await;
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }
}

macro_rules! impl_sender {
    ($name:ident, $msg_type:expr) => {
        impl Sender for $name {
            async fn send(self, svc: NotificationService) -> Result<SendResponse, Status> {
                let message_id = self.message_id.clone();
                svc.sender.send($msg_type(self)).await.map_err(|e| {
                    warn!("Failed to send message: {:?}", e);
                    Status::internal("Failed to send message")
                })?;
                Ok(SendResponse {
                    message_id,
                    timestamp: Some(to_timestamp()),
                })
            }
        }
    };
}

macro_rules! impl_into_send_request {
    ($type:ty, $msg_type:expr) => {
        impl From<$type> for Msg {
            fn from(item: $type) -> Self {
                $msg_type(item)
            }
        }

        impl From<$type> for SendRequest {
            fn from(item: $type) -> Self {
                let msg: Msg = item.into();
                SendRequest { msg: Some(msg) }
            }
        }
    };
}

impl_into_send_request!(EmailMessage, Msg::Email);
impl_sender!(EmailMessage, Msg::Email);

impl_into_send_request!(InAppMessage, Msg::InApp);
impl_sender!(InAppMessage, Msg::InApp);

impl_into_send_request!(SmsMessage, Msg::Sms);
impl_sender!(SmsMessage, Msg::Sms);

impl Deref for NotificationService {
    type Target = NotificationServiceInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

fn to_timestamp() -> Timestamp {
    let now = Utc::now();
    Timestamp {
        seconds: now.timestamp(),
        nanos: now.timestamp_subsec_nanos() as i32,
    }
}

fn dummy_send() -> mpsc::Sender<Msg> {
    let (tx, mut rx) = mpsc::channel(CHANNEL_SIZE * 100);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            info!("Sending message: {:?}", msg);
            sleep(Duration::from_millis(300)).await;
        }
    });
    tx
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use futures::StreamExt;

    use super::*;
    use crate::AppConfig;

    #[tokio::test]
    async fn send_should_work() -> Result<()> {
        let config = AppConfig::load().expect("Failed to load config");
        let svc = NotificationService::new(config);
        let stream = tokio_stream::iter(vec![
            Ok(EmailMessage::fake().into()),
            Ok(SmsMessage::fake().into()),
            Ok(InAppMessage::fake().into()),
        ]);

        let response = svc.send(stream).await?;
        let ret = response.into_inner().collect::<Vec<_>>().await;
        assert_eq!(ret.len(), 3);
        for r in ret {
            println!("{:?}", r);
        }
        Ok(())
    }
}
