use tonic::{transport::Server, Request, Response, Status};

use crm::pb::{
    user_service_server::{UserService, UserServiceServer},
    CreateUserRequest, GetUserRequest, User,
};

#[derive(Default)]
pub struct UserServer {}

#[tonic::async_trait]
impl UserService for UserServer {
    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<User>, Status> {
        let input = request.into_inner();
        println!("get_user: {:?}", input);
        Ok(Response::new(User::default()))
    }

    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<User>, Status> {
        let input = request.into_inner();
        println!("create_user: {:?}", input);
        let user = User::new(1, &input.name, &input.email);
        Ok(Response::new(user))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "[::1]:50051".parse()?;
    let svc = UserServer::default();

    Server::builder()
        .add_service(UserServiceServer::new(svc))
        .serve(addr)
        .await?;
    Ok(())
}
