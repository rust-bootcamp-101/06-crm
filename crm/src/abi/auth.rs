use chrono::{DateTime, Utc};
use jwt_simple::prelude::*;
use tonic::{service::Interceptor, Request, Status};

const JWT_ISS: &str = "crm";
const JWT_AUD: &str = "crm-client";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub ws_id: i64, // workspace_id
    pub fullname: String,
    pub email: String,
    #[serde(skip)]
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DecodingKey(Ed25519PublicKey);

impl DecodingKey {
    pub fn load(pem: &str) -> Result<Self, jwt_simple::Error> {
        Ok(Self(Ed25519PublicKey::from_pem(pem)?))
    }

    pub fn verify(&self, token: &str) -> Result<User, jwt_simple::Error> {
        let opts = VerificationOptions {
            allowed_issuers: Some(HashSet::from_strings(&[JWT_ISS])),
            allowed_audiences: Some(HashSet::from_strings(&[JWT_AUD])),
            ..Default::default()
        };
        let claims = self.0.verify_token::<User>(token, Some(opts))?;
        Ok(claims.custom)
    }
}

impl Interceptor for DecodingKey {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok());
        let user = match token {
            Some(token) => {
                let token = token
                    .strip_prefix("Bearer ")
                    .ok_or_else(|| Status::unauthenticated("invalid token"))?;
                self.verify(token)
                    .map_err(|e| Status::unauthenticated(e.to_string()))?
            }
            None => return Err(Status::unauthenticated("missing token")),
        };
        req.extensions_mut().insert(user);
        Ok(req)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use anyhow::Result;

//     #[test]
//     fn jwt_sign_verify_should_work() -> Result<()> {
//         let encoding_pem = include_str!("../../fixtures/encoding.pem");
//         let decoding_pem = include_str!("../../fixtures/decoding.pem");
//         let ek = EncodingKey::load(encoding_pem)?;
//         let dk = DecodingKey::load(decoding_pem)?;
//         let user = User::new(1, "startdusk", "startdusk@acme.org");

//         let token = ek.sign(user.clone())?;
//         let decode_user = dk.verify(&token)?;
//         assert_eq!(user, decode_user);
//         Ok(())
//     }
// }
