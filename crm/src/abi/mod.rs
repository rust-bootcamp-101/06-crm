mod auth;

pub use auth::DecodingKey;

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
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
        // SELECT name, email FROM user_stats WHERE created_at BETWEEN d1 AND d2
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

    pub async fn recall(&self, req: RecallRequest) -> Result<Response<RecallResponse>, Status> {
        let request_id = req.id;
        let last_visit_interval = make_lower_upper(req.last_visit_interval, 1);
        let last_watched_interval = make_lower_upper(req.last_watched_interval, 1);
        let query = QueryRequest::new_with_timestamps(&[
            (
                "last_visited_at",
                last_visit_interval.0,
                last_visit_interval.1,
            ),
            (
                "last_watched_at",
                last_watched_interval.0,
                last_watched_interval.1,
            ),
        ]);
        let users = self.user_stats.clone().query(query).await?.into_inner();
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
        let reqs = users.filter_map(move |user| {
            let sender = sender.clone();
            let contents = Arc::clone(&contents);
            async move {
                let user = user.ok()?;
                let req = SendRequest::new_email_msg(
                    "Recall".to_string(),
                    sender,
                    &[user.email],
                    &contents,
                );
                Some(req)
            }
        });

        self.notification.clone().send(reqs).await?;
        let ret = RecallResponse { id: request_id };
        Ok(Response::new(ret))
    }

    // 查询多少天前观看过视频的用户
    // 找出他们还未看完的视频，给他们推送消息，你有未观看完的视频
    pub async fn remind(&self, req: RemindRequest) -> Result<Response<RemindResponse>, Status> {
        let request_id = req.id;
        let last_visit_interval = make_lower_upper(req.last_visit_interval, 1);
        let query = QueryRequest::new_with_timestamps(&[(
            "last_visited_at",
            last_visit_interval.0,
            last_visit_interval.1,
        )]);
        let mut users = self.user_stats.clone().query(query).await?.into_inner();
        let sender = self.config.server.sender_email.clone();
        let (tx, rx) = mpsc::channel(1024);
        let metadata_service = self.metadata.clone();
        tokio::spawn(async move {
            while let Some(Ok(user)) = users.next().await {
                if user.started_but_not_finished.is_empty() {
                    continue;
                }
                // TODO: 此处设计/写法不好，读放大，每一个用户都得调用一次微服务查一下他未看完内容的信息
                let content_ids = user
                    .started_but_not_finished
                    .iter()
                    .map(|&x| x as u32)
                    .collect::<Vec<_>>();
                let contents = match metadata_service
                    .clone()
                    .materialize(MaterializeRequest::new_with_ids(&content_ids))
                    .await
                {
                    Ok(contents) => contents.into_inner(),
                    Err(e) => {
                        warn!("Failed to query content materialize: {:?}", e);
                        continue;
                    }
                };

                let contents: Vec<Content> = contents
                    .filter_map(|v| async move { v.ok() })
                    .collect()
                    .await;
                let tx = tx.clone();
                let sender = sender.clone();
                let req = SendRequest::new_email_msg(
                    "Remind".to_string(),
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

        self.notification.clone().send(reqs).await?;
        let ret = RemindResponse { id: request_id };
        Ok(Response::new(ret))
    }
}

fn make_lower_upper(interval: u32, days: i64) -> (DateTime<Utc>, DateTime<Utc>) {
    let dt1 = Utc::now() - Duration::days(interval as _);
    let dt2 = dt1 + Duration::days(days);
    (dt1, dt2)
}
