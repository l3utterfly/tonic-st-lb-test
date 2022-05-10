# Overview

Client & Server program to test Tonic request throughput.

# Pre-requisites

- `protobuf` compiler, see ReadMe.md file of Tonic for pre-requisites for compiling protobuf during rust build process.

# Running

Use command `-h` on server and client program to view command line arguments.

Run `server` passing in number of grpc servers to spawn. Spawned servers will listen on ports starting at 18888 to (18888 + number of grpc server). Each server is spawned in its own thread.

Client spawns N number of tokio tasks and calls the server hello world endpoint in a tight loop.

Run `client` passing in the same number of grpc server as above. Client runs with default tokio multi-threaded runtime. Pass `concurrency` argument to specify the amount of tokio tasks to spawn. `cpu` flag is used to override default number of cpu detection as it doesn't work on Windows with HyperThreading enabled.