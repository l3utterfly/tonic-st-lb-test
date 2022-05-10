use crate::hello_world::HelloRequest;
use crate::hello_world::greeter_client::GreeterClient;
use atomic_counter::AtomicCounter;
use atomic_counter::RelaxedCounter;
use tokio::runtime::Builder;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::{signal, time};
use tokio::time::Instant;
use tonic::Request;
use clap::Parser;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

/// Simple program to load test a set of grpc server endpoints
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct ClientArgs {
    /// Number of concurrent requests to start
    #[clap(short, long)]
    concurrency: usize,

    /// Number of grpc server instances that is listening, load tester will automatically distribute requests evenly to all grpc servers
    #[clap(short, long, default_value_t= 1)]
    grpc_server_num: usize,

    /// Override automatic cpu detection if needed on Windows (automatic cpu detection doesn't work well on Windows with HyperThreading)
    #[clap(long, default_value_t = 0)]
    cpus: usize,
}

fn main() {
    let args = ClientArgs::parse();

    let mut rt_builder = Builder::new_multi_thread();
    
    // base address for first listener, subsequent listeners have port number increasing sequentially
    let base_addr: SocketAddr = "127.0.0.1:18888".parse().unwrap();
    
    if args.cpus > 0 {
        rt_builder.worker_threads(args.cpus);
    }

    // starts tokio runtime
    let rt = rt_builder.enable_all().build()
        .expect("Unable to start tokio runtime");

    // start load test
    rt.block_on(async move {
        let mut tokio_tasks = Vec::new();

        println!(
            "Starting load test with concurrency: {}",
            args.concurrency
        );

        // increase this counter to shutdown, requests will check this at each loop
        let shutdown_signal_arc = Arc::new(RelaxedCounter::new(0));

        // log some statistics
        let request_count_arc = Arc::new(RelaxedCounter::new(0));
        let success_count_arc = Arc::new(RelaxedCounter::new(0));
        let error_count_arc = Arc::new(RelaxedCounter::new(0));
        
        // used to calculate instantaneous response time, these values are reset every reporting interval
        let accumulated_request_ms_arc = Arc::new(RelaxedCounter::new(0));
        let accumulated_request_count_arc = Arc::new(RelaxedCounter::new(0));

        // spawn tokio tasks equal to our concurrency setting
        for i in 0..args.concurrency {
            let mut socket_addr = base_addr.clone();

            // cycle through our grpc servers
            socket_addr.set_port(socket_addr.port() + (i % args.grpc_server_num) as u16);

            let mut client = GreeterClient::connect(format!("http://{}", socket_addr))
                .await
                .unwrap();

            let request_count_arc_clone = request_count_arc.clone();
            let success_count_arc_clone = success_count_arc.clone();
            let error_count_arc_clone = error_count_arc.clone();
            let accumulated_request_ms_arc_clone = accumulated_request_ms_arc.clone();
            let accumulated_request_count_arc_clone = accumulated_request_count_arc.clone();
            let shutdown_signal_arc_clone = shutdown_signal_arc.clone();

            tokio_tasks.push(tokio::spawn(async move {
                // log error messages to display at the end
                let mut error_msgs = Vec::new();

                loop {
                    let request = Request::new(HelloRequest {
                        name: format!("Tester {}", i)
                    });

                    request_count_arc_clone.inc();

                    let req_start = Instant::now();

                    match client.say_hello(request).await {
                        Ok(_) => {
                            accumulated_request_ms_arc_clone.add(req_start.elapsed().as_millis() as usize);

                            success_count_arc_clone.inc();
                            accumulated_request_count_arc_clone.inc();
                        }
                        Err(e) => {
                            error_count_arc_clone.inc();

                            error_msgs.push(format!("{:?}", e));
                        }
                    }

                    if shutdown_signal_arc_clone.get() > 0 {
                        // print all error messages
                        if error_msgs.len() > 0 {
                            println!("{:?}", error_msgs);
                        }
                        
                        break;
                    }
                }
            }));
        }

        tokio_tasks.push(tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(1000));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut success_rate = 100;

                        if request_count_arc.get() > 0 {
                            success_rate = ((success_count_arc.get() as f64) / (request_count_arc.get() as f64) * 100.0) as usize;
                        }

                        let mut response_time = 0;
                        let mut requests_per_second = 0;

                        if accumulated_request_count_arc.get() > 0 {
                            response_time = (accumulated_request_ms_arc.get() as f64 / accumulated_request_count_arc.get() as f64) as usize;
                            requests_per_second = accumulated_request_count_arc.get();
                        }

                        println!(
                            "Total: {}; Success: {}; Error: {}; Success rate: {}%; Requests/s: {}; Response time: {}ms",
                            request_count_arc.get(), success_count_arc.get(), error_count_arc.get(),
                            success_rate, requests_per_second, response_time
                        );

                        accumulated_request_ms_arc.reset();
                        accumulated_request_count_arc.reset();
                    }
                    _ = signal::ctrl_c() => {
                        break;
                    }
                }
            }
        }));

        // wait for shutdown signal
        signal::ctrl_c().await.expect("Unable to handle shutdown signal");
        println!("Received shutdown signal. Exiting, waiting for all thread to complete...");

        // increase this counter, all threads will check this at every loop
        shutdown_signal_arc.inc();

        futures::future::join_all(tokio_tasks).await;

        println!("All done.")
    });
}
