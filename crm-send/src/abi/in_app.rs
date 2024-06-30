#[allow(unused)]
use crate::pb::InAppMessage;

#[cfg(test)]
impl InAppMessage {
    pub fn fake() -> Self {
        use uuid::Uuid;

        InAppMessage {
            message_id: Uuid::new_v4().to_string(),
            device_id: Uuid::new_v4().to_string(),
            title: "Hello".to_string(),
            body: "Hello world".to_string(),
        }
    }
}
