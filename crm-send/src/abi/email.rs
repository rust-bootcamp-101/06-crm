#[allow(unused)]
use crate::pb::EmailMessage;

#[cfg(test)]
impl EmailMessage {
    pub fn fake() -> Self {
        use fake::{faker::internet::en::SafeEmail, Fake};
        use uuid::Uuid;

        EmailMessage {
            message_id: Uuid::new_v4().to_string(),
            sender: SafeEmail().fake(),
            recipients: vec![SafeEmail().fake()],
            subject: "Hello".to_string(),
            body: "Hello world".to_string(),
        }
    }
}
