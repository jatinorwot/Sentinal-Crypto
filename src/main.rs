use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::connect_async;
use futures_util::stream::StreamExt;
use std::fs::File;
use std::io::Write;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct CacheData {
    average_price: f64,
    prices: Vec<f64>,
}

static mut RX_CHANNEL: Option<mpsc::Receiver<SignedMessage>> = None;

#[derive(Debug, Serialize, Deserialize)]
struct SignedMessage {
    message: String,
    signature: Vec<u8>,
}

impl SignedMessage {
    fn new(message: String, signature: Vec<u8>) -> Self {
        SignedMessage { message, signature }
    }
}

async fn client_process(tx: mpsc::Sender<SignedMessage>, mode: &str, times: u64) {
    let uri = "wss://stream.binance.com:9443/ws/btcusdt@trade";
    let mut prices = Vec::new();

    match connect_async(uri).await {
        Ok((socket, _)) => {
            let mut socket = socket;

            // Sleep until 10:01:01 AM
            let delay_duration = Duration::from_secs(1);
            sleep(delay_duration).await;

            for _ in 0..times {
                if let Some(Ok(message)) = socket.next().await {
                    match serde_json::from_slice::<serde_json::Value>(&message.into_data()) {
                        Ok(data) => {
                            if let (Some(symbol), Some(price)) = (data["s"].as_str(), data["p"].as_str()) {
                                if symbol == "btcusdt" {
                                    if let Ok(price) = price.parse::<f64>() {
                                        prices.push(price);
                                        println!("Cache Mode: {:?}", data);
                                    }
                                }
                            }
                        }
                        Err(err) => eprintln!("Failed to deserialize message: {}", err),
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to connect to WebSocket: {}", err);
            return;
        }
    }

    if !prices.is_empty() {
        let average_price: f64 = prices.iter().sum::<f64>() / prices.len() as f64;
        let signed_message = SignedMessage::new(
            format!("Cache complete. The average USD price of BTC is: {}", average_price),
            Vec::new(),
        );
        tx.send(signed_message).await.unwrap();
    } else {
        println!("No data points received in cache mode.");
    }
}

async fn aggregator_process(mut rx: mpsc::Receiver<SignedMessage>) {
    let mut received_values = Vec::new();

    for _ in 0..5 {
        if let Some(value) = rx.recv().await {
            received_values.push(value);
        }
    }

    // Validate signatures and calculate the average
    let mut valid_averages = Vec::new();
    for signed_message in received_values {
        // Implement signature validation logic here
        // ...

        // For now, we assume all signatures are valid
        valid_averages.push(
            signed_message
                .message
                .split_whitespace()
                .last()
                .unwrap()
                .parse::<f64>()
                .unwrap(),
        );
    }

    let final_average: f64 = valid_averages.iter().sum::<f64>() / valid_averages.len() as f64;
    println!("Aggregator: Final average USD price of BTC is: {}", final_average);

    let aggregate_data = CacheData {
        average_price: final_average,
        prices: valid_averages,
    };

    if let Ok(json_data) = serde_json::to_string(&aggregate_data) {
        if let Ok(mut file) = File::create("aggregate_data.json") {
            if let Err(err) = file.write_all(json_data.as_bytes()) {
                eprintln!("Failed to write to output file: {}", err);
            }
        } else {
            eprintln!("Failed to create output file.");
        }
    } else {
        eprintln!("Failed to serialize CacheData.");
    }
}

async fn read_mode() {
    if let Ok(file) = tokio::fs::read_to_string("aggregate_data.json").await {
        if let Ok(cache_data) = serde_json::from_str::<CacheData>(&file) {
            println!("Read Mode: Average USD price of BTC is: {}", cache_data.average_price);
            println!("Read Mode: Data points: {:?}", cache_data.prices);
        } else {
            println!("Failed to deserialize data from input file.");
        }
    } else {
        println!("File aggregate_data.json not found.");
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    if args.len() < 3 {
        eprintln!("Usage: ./simple --mode=cache OR ./simple --mode=read");
        std::process::exit(1);
    }

    let mode = &args[2];
    match mode.as_str() {
        "--mode=cache" => {
            let times = args.get(4).and_then(|t| t.parse::<u64>().ok()).unwrap_or(10);
            cache_mode(times).await;
        }
        "--mode=read" => {
            read_mode().await;
        }
        _ => {
            eprintln!("Invalid mode. Use '--mode=cache' or '--mode=read'.");
            std::process::exit(1);
        }
    }
}

async fn cache_mode(times: u64) {
    let (tx, rx) = mpsc::channel::<SignedMessage>(5);
    unsafe { RX_CHANNEL = Some(rx); }

    let mut client_handles = Vec::new();
    for _ in 0..5 {
        let tx_clone = tx.clone();
        let handle = tokio::spawn(async move {
            client_process(tx_clone, "--mode=cache", times).await;
        });
        client_handles.push(handle);
    }

    let aggregator_handle = tokio::spawn(aggregator_process(unsafe { RX_CHANNEL.take().unwrap() }));

    // Sleep for `times` seconds
    sleep(Duration::from_secs(times)).await;

    // Wait for all client processes to finish
    for handle in client_handles {
        handle.await.unwrap();
    }

    // Wait for the aggregator process to finish
    aggregator_handle.await.unwrap();
}
