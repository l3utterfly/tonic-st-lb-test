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

    let mut threads = Vec::new();

    for i in 0..args.num {
        // increase port number for each server instance
        let mut addr = base_addr.clone();

        addr.set_port(base_addr.port() + (i as u16));

        let thread_builder = std::thread::Builder::new().name(format!("GRPC server #{}", addr.port()));

        threads.push(thread_builder.spawn(move || {
            let greeter = MyGreeter::default();

            let mut tokio_runtime_builder = tokio::runtime::Builder::new_current_thread();

            let rt = tokio_runtime_builder
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime.");

            let server_future = Server::builder()
                .add_service(GreeterServer::new(greeter))
                .serve_with_shutdown(addr, async {
                    signal::ctrl_c()
                        .await
                        .expect("failed to listen for ctrl+c event");

                    println!("GRPC service #{}: shutdown signal received, goodbye!", addr.port());
                });

            println!("GRPC server #{} started.", addr.port());

            rt.block_on(server_future)
                .expect("failed to successfully run the future on RunTime");
        }).expect("Unable to spawn thread."));
    }

    // join all threads
    for h in threads {
        let thread_id = h.thread().id();

        h.join()
            .expect(&format!("Unable to join thread {:?}.", thread_id));
    }

    println!("All done!");
}
