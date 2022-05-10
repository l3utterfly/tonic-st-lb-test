use std::net::SocketAddr;
use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use tokio::signal;
use tonic::{transport::Server, Request, Response, Status};
use clap::Parser;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(reply))
    }
}

/// Simple grpc server program
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct ClientArgs {
    /// Number of grpc server instances to spawn. Each server instance is spawned using a single threaded runtime
    #[clap(short, long, default_value_t= 1)]
    num: usize,
}

fn main() {
    let args = ClientArgs::parse();

    let base_addr: SocketAddr = "0.0.0.0:18888".parse().unwrap();

    let greeter = MyGreeter::default();

    let mut tokio_runtime_builder = tokio::runtime::Builder::new_multi_thread();

    let rt = tokio_runtime_builder
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime.");

    let server_future = Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve_with_shutdown(base_addr, async {
            signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl+c event");

            println!("GRPC service #{}: shutdown signal received, goodbye!", base_addr.port());
        });

    println!("GRPC server #{} started.", base_addr.port());

    rt.block_on(server_future)
        .expect("failed to successfully run the future on RunTime");

    println!("All done!");
}
