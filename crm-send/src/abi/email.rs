use crm_metadata::{pb::Content, Tpl};
use uuid::Uuid;

use crate::pb::{send_request::Msg, EmailMessage, SendRequest};

impl SendRequest {
    pub fn new_email_msg(
        subject: String,
        sender: String,
        recipients: &[String],
        contents: &[Content],
    ) -> Self {
        let tpl = Tpl(contents);
        let msg = Msg::Email(EmailMessage {
            message_id: Uuid::new_v4().to_string(),
            subject,
            sender,
            recipients: recipients.to_vec(),
            body: tpl.to_body(),
        });

        Self { msg: Some(msg) }
    }
}

#[cfg(feature = "test_utils")]
impl EmailMessage {
    pub fn fake() -> Self {
        use fake::{faker::internet::en::SafeEmail, Fake};

        EmailMessage {
            message_id: Uuid::new_v4().to_string(),
            sender: SafeEmail().fake(),
            recipients: vec![SafeEmail().fake()],
            subject: "Hello".to_string(),
            body: "Hello world".to_string(),
        }
    }
}
