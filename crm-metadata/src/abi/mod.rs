use std::collections::HashSet;

use fake::{
    faker::{chrono::en::DateTimeBetween, lorem::en::Sentence, name::zh_cn::Name},
    Fake, Faker,
};
use futures::{stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

use chrono::{DateTime, Days, Utc};
use futures::Stream;
use prost_types::Timestamp;
use rand::Rng;
use tokio::sync::mpsc;

use crate::{
    pb::{Content, MaterializeRequest, Publisher},
    MetadataService, ResponseStream, ServiceResult,
};

const CHANNEL_SIZE: usize = 1024;

impl MetadataService {
    pub async fn materialize(
        &self,
        mut stream: impl Stream<Item = Result<MaterializeRequest, Status>> + Send + 'static + Unpin,
    ) -> ServiceResult<ResponseStream> {
        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
        tokio::spawn(async move {
            while let Some(Ok(req)) = stream.next().await {
                let content = Content::materialize(req.id);
                let _ = tx.send(Ok(content)).await;
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }
}

impl MaterializeRequest {
    pub fn new_with_ids(ids: &[u32]) -> impl Stream<Item = Self> {
        let reqs: HashSet<_> = ids.iter().map(|id| Self { id: *id }).collect();
        stream::iter(reqs)
    }
}

impl Content {
    pub fn materialize(id: u32) -> Self {
        let mut rng = rand::thread_rng();
        Content {
            id,
            name: Name().fake(),
            description: Sentence(3..7).fake(),
            publishers: (1..rng.gen_range(2..10))
                .map(|_| Publisher::new())
                .collect(),
            url: "https:://placehold.co/1600x900".to_string(),
            image: "https:://placehold.co/1600x900".to_string(),
            r#type: Faker.fake(),
            created_at: created_at(),
            views: rng.gen_range(123432..10000000),
            likes: rng.gen_range(1234..100000),
            dislikes: rng.gen_range(123..10000),
        }
    }

    pub fn to_body(&self) -> String {
        format!("Content: {:?}", self)
    }
}

pub struct Tpl<'a>(pub &'a [Content]);

impl<'a> Tpl<'a> {
    pub fn to_body(&self) -> String {
        format!("Tpl: {:?}", self.0)
    }
}

impl Publisher {
    pub fn new() -> Self {
        Self {
            id: (10000..2000000).fake(),
            name: Name().fake(),
            avatar: "https:://placehold.co/400x400".to_string(),
        }
    }
}

fn before(days: u64) -> DateTime<Utc> {
    Utc::now().checked_sub_days(Days::new(days)).unwrap()
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn created_at() -> Option<Timestamp> {
    let date: DateTime<Utc> = DateTimeBetween(before(365), now()).fake();
    Some(Timestamp {
        seconds: date.timestamp(),
        nanos: date.timestamp_subsec_nanos() as i32,
    })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use futures::StreamExt;

    use super::*;
    use crate::AppConfig;

    #[tokio::test]
    async fn materialize_should_work() -> Result<()> {
        let config = AppConfig::load().expect("Failed to load config");
        let svc = MetadataService::new(config);
        let stream = tokio_stream::iter(vec![
            Ok(MaterializeRequest { id: 1 }),
            Ok(MaterializeRequest { id: 2 }),
            Ok(MaterializeRequest { id: 3 }),
        ]);

        let response = svc.materialize(stream).await?;
        let ret = response.into_inner().collect::<Vec<_>>().await;
        assert_eq!(ret.len(), 3);
        for r in ret {
            println!("{:?}", r);
        }
        Ok(())
    }
}
