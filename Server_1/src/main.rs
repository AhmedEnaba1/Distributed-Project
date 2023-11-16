// // use async_std::net::UdpSocket;
// // use std::net::SocketAddr;
// // use tokio::time::sleep;
// // use std::time::Duration;

// // async fn server_task(mut server_socket: UdpSocket) {
// //     let mut buffer = [0; 1024];

// //     loop {
// //         if let Ok((bytes_received, client_address)) = server_socket.recv_from(&mut buffer).await {
// //             let request = String::from_utf8_lossy(&buffer[..bytes_received]);
// //             println!("Server received request from {}: {}", client_address, request);

// //             // Simulate some processing (you should replace this with your actual server logic)
// //             sleep(Duration::from_millis(500)).await;

// //             // Echo the received request back to the client
// //             server_socket
// //                 .send_to(&buffer[..bytes_received], client_address)
// //                 .await
// //                 .expect("Failed to send response to client");

// //             // Clear the buffer for the next request
// //             buffer = [0; 1024];
// //         }
// //     }
// // }

// // #[tokio::main]
// // async fn main() {
// //     let server_address: SocketAddr = "127.0.0.2:21112".parse().expect("Failed to parse server address");
// //     let server_socket = UdpSocket::bind(&server_address)
// //         .await
// //         .expect("Failed to bind server socket");

// //     tokio::spawn(server_task(server_socket));

// //     loop {
// //         // Keep the main thread alive
// //         tokio::time::sleep(Duration::from_secs(1)).await;
// //     }
// // }


use async_std::net::UdpSocket;
use std::net::SocketAddr;
use tokio::time::sleep;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use serde_json::Result;

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Election(usize),
    Answer(usize),
    Coordinator(usize),
    Request(String),
}

async fn server_task(mut server_socket: UdpSocket, server_id: usize) {
    let mut buffer = [0; 1024];
    let mut is_coordinator = false;

    loop {
        if let Ok((bytes_received, client_address)) = server_socket.recv_from(&mut buffer).await {
            let received_message: Message =
                serde_json::from_slice(&buffer[..bytes_received]).expect("Failed to deserialize message");
            println!("Message: {:?}", received_message);
            match received_message {
                Message::Election(initiating_server_id) => {
                    if initiating_server_id > server_id {
                        // Respond to the election with an Answer
                        let answer_message = Message::Answer(server_id);
                        server_socket
                            .send_to(
                                &serde_json::to_string(&answer_message).unwrap().as_bytes(),
                                client_address,
                            )
                            .await
                            .expect("Failed to send Answer message to client");

                        // Start a new election
                        start_election(&server_socket, server_id).await;
                    }
                }
                Message::Answer(coordinator_id) => {
                    // Handle Answer message
                    println!("Received Answer from Server {}", coordinator_id);
                }
                Message::Coordinator(new_coordinator_id) => {
                    // Handle Coordinator message
                    println!(
                        "Received Coordinator message. Server {} is the new coordinator.",
                        new_coordinator_id
                    );
                    is_coordinator = new_coordinator_id == server_id;
                }
                Message::Request(request) => {
                    // Handle the actual request from the middleware
                    println!("Server {} received request: {}", server_id, request);

                    // Simulate processing the request
                    sleep(Duration::from_millis(500)).await;

                    // Respond to the middleware (assuming the middleware's address is known)
                    let response_message = Message::Request(format!("Response from Server {}", server_id));
                    server_socket
                        .send_to(
                            &serde_json::to_string(&response_message).unwrap().as_bytes(),
                            client_address,
                        )
                        .await
                        .expect("Failed to send response to client");
                }
            }

            // Clear the buffer for the next message
            buffer = [0; 1024];
        }
    }
}

async fn start_election(mut server_socket: &UdpSocket, server_id: usize) {
    println!("Server {} is starting an election.", server_id);

    // Broadcast Election messages to servers with higher IDs
    let election_message = Message::Election(server_id);
    server_socket
        .send_to(
            &serde_json::to_string(&election_message).unwrap().as_bytes(),
            "127.0.0.2:21112", // Replace with actual addresses of other servers
        )
        .await
        .expect("Failed to send Election message");

    // Wait for Answers from servers with higher IDs
    let mut buffer = [0; 1024];
    let mut highest_responder_id: Option<usize> = None;

    for _ in 0..2 {
        if let Ok((bytes_received, _)) = server_socket.recv_from(&mut buffer).await {
            let received_message: Message =
                serde_json::from_slice(&buffer[..bytes_received]).expect("Failed to deserialize message");

            if let Message::Answer(responder_id) = received_message {
                if let Some(current_highest_responder_id) = highest_responder_id {
                    if responder_id > current_highest_responder_id {
                        highest_responder_id = Some(responder_id);
                    }
                } else {
                    highest_responder_id = Some(responder_id);
                }
            }

            // Clear the buffer for the next message
            buffer = [0; 1024];
        }
    }

    // If no higher responder, declare the current server as the coordinator
    if let Some(coordinator_id) = highest_responder_id {
        let coordinator_message = Message::Coordinator(coordinator_id);
        server_socket
            .send_to(
                &serde_json::to_string(&coordinator_message).unwrap().as_bytes(),
                "127.0.0.2:21112", // Replace with actual addresses of other servers
            )
            .await
            .expect("Failed to send Coordinator message");

        println!("Server {} is the new coordinator.", server_id);
    }
}

#[tokio::main]
async fn main() {
    let server_id = 2; // Replace with the actual ID of the server
    let server_address: SocketAddr = format!("127.0.0.{}:21112", server_id)
        .parse()
        .expect("Failed to parse server address");
    let server_socket = UdpSocket::bind(&server_address)
        .await
        .expect("Failed to bind server socket");

    tokio::spawn(server_task(server_socket, server_id));

    loop {
        // Keep the main thread alive
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}