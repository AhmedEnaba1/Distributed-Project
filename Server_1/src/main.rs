use async_std::net::UdpSocket;
use std::{net::SocketAddr, sync::Arc};
use tokio::time::sleep;
use std::time::Duration;
use serde::{Serialize, Deserialize, de::Error};
use serde_json::Result;

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Election(usize),
    Answer(usize),
    Coordinator(usize),
    Request(String),
}

async fn server_task(server1_id: usize) {
    let server_address: SocketAddr = format!("127.0.0.1:8080")
        .parse()
        .expect("Failed to parse server address");

    // Wrap the UdpSocket in Arc
    let client_socket = Arc::new(UdpSocket::bind(&server_address)
        .await
        .expect("Failed to bind server socket"));

    let mut buffer = [0; 1024];

    loop {
        if let Ok((bytes_received, client_address)) = client_socket.recv_from(&mut buffer).await {
            println!("Source: {}", client_address);
            let received_message: Message =
                serde_json::from_slice(&buffer[..bytes_received]).expect("Failed to deserialize message");
            println!("Message: {:?}", received_message);

            match received_message {
                Message::Request(request) => {
                    // Handle the actual request from the middleware
                    println!("Server {} received request: {}", server1_id, request);

                    // Start a new election
                    let client_socket_clone = Arc::clone(&client_socket);
                    let client_address_clone = client_address;
                    let leader = start_election(server1_id).await;
                    tokio::spawn(async move {
                        //let leader = start_election(&client_socket_clone, client_address_clone, server1_id).await;
                        if leader {
                            process_request(&client_socket_clone, client_address_clone).await;
                        }
                    });
                }
                _ => {}
            }

            // Clear the buffer for the next message
            buffer = [0; 1024];
        }
    }
}

async fn start_election(server1_id: usize) -> bool {
    println!("Server 1 is starting an election.");

    // Wrap the UdpSocket in Arc
    let socket2 = Arc::new(UdpSocket::bind("127.0.0.1:2112")
        .await
        .expect("Failed to bind server socket"));

    // Wrap the UdpSocket in Arc
    let socket3 = Arc::new(UdpSocket::bind("127.0.0.1:2113")
        .await
        .expect("Failed to bind server socket"));

    // Broadcast Election messages to servers with higher IDs
    let election_message = Message::Election(server1_id);

    // Use Arc to share ownership without requiring Clone
    socket2.send_to(
        &serde_json::to_string(&election_message).unwrap().as_bytes(),
        "127.0.0.2:2112", // Replace with actual addresses of other servers
    ).await.expect("Failed to send Election message");

    socket3.send_to(
        &serde_json::to_string(&election_message).unwrap().as_bytes(),
        "127.0.0.3:2112", // Replace with actual addresses of other servers
    ).await.expect("Failed to send Election message");

    // Set a timeout for receiving messages
    let timeout = Duration::from_millis(200);

    // Update the server IDs
    let server2_id = receive_election_message(Arc::clone(&socket2), timeout).await.unwrap_or(0);
    let server3_id = receive_election_message(Arc::clone(&socket3), timeout).await.unwrap_or(0);

    println!("IDs: {},{}", server2_id, server3_id);

    // Check if the current server is the Leader or not
    let leader = server1_id > server2_id && server1_id > server3_id;
    if leader {
        // Call a function to process the request (e.g., become the coordinator)
        println!("This Server is the Coordinator!");
    }

    leader
}

async fn receive_election_message(socket: Arc<UdpSocket>, timeout: Duration) -> Result<usize> {
    let mut buffer = [0; 1024];
    let mut server_id: usize = 0;

    // Use Arc to share ownership without requiring Clone
    let result = tokio::time::timeout(timeout, async {
        if let Ok((bytes_received, _)) = socket.recv_from(&mut buffer).await {
            let received_message: Message =
                serde_json::from_slice(&buffer[..bytes_received]).expect("Failed to deserialize message");

            if let Message::Election(id) = received_message {
                server_id = id;
            }
        }
    })
        .await;

    result.map(|_| server_id).map_err(|_| serde_json::Error::custom("Timeout occurred"))
}

async fn process_request(client_socket: &Arc<UdpSocket>, client_address: SocketAddr) {
    // Simulate processing the request
    sleep(Duration::from_millis(5000)).await;

    // Respond to the middleware (assuming the middleware's address is known)
    let response_message = Message::Request(format!("Response from Server 1"));

    // Use Arc to share ownership without requiring Clone
    client_socket.send_to(
        &serde_json::to_string(&response_message).unwrap().as_bytes(),
        client_address, // Replace with the actual client address
    ).await.expect("Failed to send response to client");
}

#[tokio::main]
async fn main() {
    let server1_id = 1; // Replace with the actual ID of the server

    let task = server_task(server1_id);
    let _ = tokio::join!(task);
}
