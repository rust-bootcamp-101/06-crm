use tonic::Request;

use crm::pb::{user_service_client::UserServiceClient, CreateUserRequest, GetUserRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "http://[::1]:50051";
    let mut client = UserServiceClient::connect(addr).await?;

    let req = Request::new(CreateUserRequest {
        name: "benjamin".to_string(),
        email: "benjamin@acme.org".to_string(),
    });
    let resp = client.create_user(req).await?;
    println!("RESPONSE={:?}", resp);

    let req = Request::new(GetUserRequest { id: 1 });
    let resp = client.get_user(req).await?;
    println!("RESPONSE={:?}", resp);

    Ok(())
}
