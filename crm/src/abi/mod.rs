mod auth;

pub use auth::DecodingKey;

use std::sync::Arc;

use chrono::{Duration, Utc};
use crm_metadata::pb::{Content, MaterializeRequest};
use crm_send::pb::SendRequest;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::warn;
use user_stat::pb::QueryRequest;

use crate::{
    pb::{
        RecallRequest, RecallResponse, RemindRequest, RemindResponse, WelcomeRequest,
        WelcomeResponse,
    },
    CrmService,
};

impl CrmService {
    pub async fn welcome(&self, req: WelcomeRequest) -> Result<Response<WelcomeResponse>, Status> {
        let request_id = req.id;
        let dt1 = Utc::now() - Duration::days(req.interval as _);
        let dt2 = dt1 + Duration::days(1);
        let query = QueryRequest::new_with_date("created_at", dt1, dt2);
        let mut users = self.user_stats.clone().query(query).await?.into_inner();
        let contents = self
            .metadata
            .clone()
            .materialize(MaterializeRequest::new_with_ids(&req.content_ids))
            .await?
            .into_inner();
        let contents: Vec<Content> = contents
            .filter_map(|v| async move { v.ok() })
            .collect()
            .await;
        let contents = Arc::new(contents);
        let sender = self.config.server.sender_email.clone();
        // 下面两种写法都可以(用来作为学习)
        let (tx, rx) = mpsc::channel(1024);
        tokio::spawn(async move {
            while let Some(Ok(user)) = users.next().await {
                let contents = Arc::clone(&contents);
                let tx = tx.clone();
                let sender = sender.clone();
                let req = SendRequest::new_email_msg(
                    "Welcome".to_string(),
                    sender,
                    &[user.email],
                    &contents,
                );
                if let Err(e) = tx.send(req).await {
                    warn!("Failed to send message: {:?}", e)
                }
            }
        });
        let reqs = ReceiverStream::new(rx);

        // let reqs = users.filter_map(move |user| {
        //     let sender = sender.clone();
        //     let contents = Arc::clone(&contents);
        //     async move {
        //         let user = user.ok()?;
        //         let req = gen_send_req("Welcome".to_string(), sender, user, &contents);
        //         Some(req)
        //     }
        // });

        self.notification.clone().send(reqs).await?;
        let ret = WelcomeResponse { id: request_id };
        Ok(Response::new(ret))
    }

    // TODO: homework
    pub async fn recall(&self, _req: RecallRequest) -> Result<Response<RecallResponse>, Status> {
        todo!()
    }

    // TODO: homework
    pub async fn remind(&self, _req: RemindRequest) -> Result<Response<RemindResponse>, Status> {
        todo!()
    }
}
