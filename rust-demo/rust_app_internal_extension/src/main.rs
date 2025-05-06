use std::collections::HashMap;

use aws_lambda_events::apigw::ApiGatewayProxyRequest;
use lambda_extension::Extension;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::signal::unix::{signal, SignalKind};

/// This is a made-up example. Requests come into the runtime as unicode
/// strings in json format, which can map to any structure that implements `serde::Deserialize`
/// The runtime pays no attention to the contents of the request payload.
#[derive(Deserialize)]
struct Request {}

/// This is a made-up example of what a response structure may look like.
/// There is no restriction on what it can be. The runtime requires responses
/// to be serialized into json. The runtime pays no attention
/// to the contents of the response payload.
#[derive(Serialize)]
struct Response {
    statusCode: i32,
    body: String,
}

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
/// - https://github.com/aws-samples/serverless-rust-demo/
async fn function_handler(event: LambdaEvent<ApiGatewayProxyRequest>) -> Result<Response, Error> {
    // Prepare the response payload
    let mut payload = HashMap::new();
    let source_ip = &*(event
        .payload
        .request_context
        .identity
        .source_ip
        .unwrap()
        .to_string());
    payload.insert("message", "hello rust");
    payload.insert("source ip", source_ip);
    payload.insert("architecture", std::env::consts::ARCH);
    payload.insert("operating system", std::env::consts::OS);
    // Prepare the response
    let body_content = json!(payload).to_string();
    let resp = Response {
        statusCode: 200,
        body: body_content,
    };

    // Return `Response` (it will be serialized to JSON automatically by the runtime)
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    // You need an extension registered with the Lambda orchestrator in order for your process
    // to receive a SIGTERM for graceful shutdown.
    //
    // We accomplish this here by registering a no-op internal extension, which doesn't subscribe to any events.
    //
    // You could also run a useful internal extension, such as in:
    // https://github.com/awslabs/aws-lambda-rust-runtime/blob/main/examples/extension-internal-flush
    let extension = Extension::new()
        // Don't subscribe to any events
        .with_events(&[])
        // Internal extension names MUST be unique within a given Lambda function.
        .with_extension_name("no-op")
        // Extensions MUST be registered before calling lambda_runtime::run(), which ends the Init
        // phase and begins the Invoke phase.
        .register()
        .await
        .expect("could not register extension");

    // Handle SIGTERM signal:
    // https://tokio.rs/tokio/topics/shutdown
    // https://rust-cli.github.io/book/in-depth/signals.html
    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _sigint = sigint.recv() => {
                println!("[runtime] SIGINT received");
                println!("[runtime] Graceful shutdown in progress ...");
                println!("[runtime] Graceful shutdown completed");
                std::process::exit(0);
            },
            _sigterm = sigterm.recv()=> {
                println!("[runtime] SIGTERM received");
                println!("[runtime] Graceful shutdown in progress ...");
                println!("[runtime] Graceful shutdown completed");
                std::process::exit(0);
            },
        }
    });

    // TODO: add biased! to always poll the handler future first, once supported:
    // https://github.com/tokio-rs/tokio/issues/7304
    tokio::try_join!(
        lambda_runtime::run(service_fn(function_handler)),
        extension.run(),
    )?;

    Ok(())
}
