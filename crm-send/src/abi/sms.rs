#[allow(unused)]
use crate::pb::SmsMessage;

#[cfg(test)]
impl SmsMessage {
    pub fn fake() -> Self {
        use fake::{faker::phone_number::en::PhoneNumber, Fake};
        use uuid::Uuid;

        SmsMessage {
            message_id: Uuid::new_v4().to_string(),
            sender: PhoneNumber().fake(),
            recipients: vec![PhoneNumber().fake()],
            subject: "Hello world".to_string(),
        }
    }
}
