use async_std::net::UdpSocket;
use serde::{Deserialize, Serialize};
use serde_json;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fs, thread};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;
use tokio::time::sleep;
use tokio::time::timeout;

#[derive(Debug, Serialize, Deserialize)]
struct frag {
    total_frags_number: usize,
    position: i16,
    packet: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Request(String),
}

async fn middleware_task(mut middleware_socket: UdpSocket) {
    let server_addresses = ["127.0.0.1:8080", "127.0.0.2:8080"];
    let mut buffer = [0; 65536];
    let mut ack_buffer = [0; 1024];
    let mut ack = false;
    let mut replier_server: SocketAddr = "127.0.0.1:8080".parse().expect("Failed");

    loop {
        if let Ok((bytes_received, client_address)) = middleware_socket.recv_from(&mut buffer).await
        {
            let server_socket = UdpSocket::bind("127.0.0.8:8080")
                .await
                .expect("Failed to bind server socket");
            let received_message: Message = serde_json::from_slice(&buffer[..bytes_received])
                .expect("Failed to deserialize message");
            let request = match received_message {
                Message::Request(request_string) => request_string,
            };
            let frag: frag = serde_json::from_str(&request).unwrap();
            if (frag.position == 0) && (!ack) {
                println!("hena");
                let se = serde_json::to_string(&frag).unwrap();
                let request_message = Message::Request(se);
                let serialized_request =
                    serde_json::to_string(&request_message).expect("Failed to serialize request");

                for server_address in &server_addresses {
                    let server_address: SocketAddr = server_address
                        .parse()
                        .expect("Failed to parse server address");

                    server_socket
                        .send_to(serialized_request.as_bytes(), &server_address)
                        .await
                        .expect("Failed to send data to server");
                }

                if let Ok((_, replier)) = server_socket.recv_from(&mut ack_buffer).await {
                    ack = true;
                    replier_server = replier;
                };

                middleware_socket
                    .send_to(&ack_buffer, client_address)
                    .await
                    .expect("Failed to send acknowledgment to client");

                // Sleep to give time for the server to send the acknowledgment
                sleep(Duration::from_millis(10)).await;

                // Clear the buffer for the next request
                buffer = [0; 65536];
                ack_buffer = [0; 1024];
            } else {
                if (frag.position==-1){
                    ack =false;
                }
                let se = serde_json::to_string(&frag).unwrap();
                let request_message = Message::Request(se);
                let serialized_request =
                    serde_json::to_string(&request_message).expect("Failed to serialize request");
                server_socket
                    .send_to(serialized_request.as_bytes(), &replier_server)
                    .await
                    .expect("Failed to send data to server");
                server_socket.recv_from(&mut ack_buffer).await;

                middleware_socket
                    .send_to(&ack_buffer, client_address)
                    .await
                    .expect("Failed to send acknowledgment to client");

                // Sleep to give time for the server to send the acknowledgment
                sleep(Duration::from_millis(10)).await;

                // Clear the buffer for the next request
                buffer = [0; 65536];
                ack_buffer = [0; 1024];
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let middleware_address: SocketAddr = "127.0.0.8:12345"
        .parse()
        .expect("Failed to parse middleware address");
    let client_socket = UdpSocket::bind("127.0.0.8:0")
        .await
        .expect("Failed to bind client socket");

    let middleware_socket = UdpSocket::bind(&middleware_address)
        .await
        .expect("Failed to bind middleware socket");

    tokio::spawn(middleware_task(middleware_socket));
    let termination = Arc::new(Mutex::new(0));
    let termination_clone = Arc::clone(&termination);

    let (tx, _) = tokio::sync::broadcast::channel::<()>(1);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");

    while *termination.lock().unwrap() == 0 {
        println!("Press Enter to send a Request");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        if input.trim() == "" {
            for i in 1..2 {
                if (i % 90 == 0) {
                    sleep(Duration::from_millis(10)).await;
                }

                let picture_data = fs::read("japan.png").expect("Failed to read image!");
                let frags = (picture_data.len() / 16384) + 1;

                for (index, piece) in picture_data.chunks(16384).enumerate() {
                    let end = index == frags - 1;
                    let frag = frag {
                        packet: piece.to_vec(),
                        position: if end { -1 } else { index.try_into().unwrap() },
                        total_frags_number: frags,
                    };

                    let se = serde_json::to_string(&frag).unwrap();
                    let request_message = Message::Request(se);
                    let serialized_request = serde_json::to_string(&request_message)
                        .expect("Failed to serialize request");

                    client_socket
                        .send_to(serialized_request.as_bytes(), middleware_address)
                        .await
                        .expect("Failed to send request to middleware");
                    println!("Waiting for response.");

                    // Receive response from the server (the first one)
                    let mut client_buffer = [0; 65536];
                    client_socket
                        .recv_from(&mut client_buffer)
                        .await
                        .expect("Failed to receive response from server");

                    let response = String::from_utf8_lossy(&client_buffer);
                    println!("Client received response from server: {}", response);
                }
            }
        }
    }
}
