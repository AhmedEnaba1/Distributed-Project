use async_std::net::UdpSocket;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;
use std::thread;
use std::sync::{Arc, Mutex};
use tokio::time::timeout;
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Request(String),
}

async fn middleware_task(mut middleware_socket: UdpSocket) {
    let server_addresses = ["127.0.0.1:8080", "127.0.0.2:8080"];
    let mut buffer = [0; 1024];
    let mut ack_buffer = [0; 1024];

    loop {
        if let Ok((_bytes_received, client_address)) = middleware_socket.recv_from(&mut buffer).await {
            let server_socket = UdpSocket::bind("127.0.0.7:8080")
                .await
                .expect("Failed to bind server socket");

            for server_address in &server_addresses {
                let server_address: SocketAddr = server_address.parse().expect("Failed to parse server address");

                let request_message = Message::Request("Request from Client 2!".to_string());
                let serialized_request = serde_json::to_string(&request_message).expect("Failed to serialize request");

                server_socket
                    .send_to(serialized_request.as_bytes(), &server_address)
                    .await
                    .expect("Failed to send data to server");
            }

            let timeout_duration = Duration::from_secs(7);

            server_socket.recv_from(&mut ack_buffer).await;

            middleware_socket
                .send_to(&ack_buffer, client_address)
                .await
                .expect("Failed to send acknowledgment to client");

            // Sleep to give time for the server to send the acknowledgment
            sleep(Duration::from_millis(10)).await;

            // Clear the buffer for the next request
            buffer = [0; 1024];
            ack_buffer = [0; 1024];
        }
    }
}

#[tokio::main]
async fn main() {
    let dos_address= "127.0.0.255:12345";
    let middleware_address: SocketAddr = "127.0.0.7:12345".parse().expect("Failed to parse middleware address");
    let client_socket = UdpSocket::bind("127.0.0.7:0").await.expect("Failed to bind client socket");
    //let client_socket_register = UdpSocket::bind("127.0.0.8:8090").await.expect("Failed to bind client socket");
    //let client_socket_query = UdpSocket::bind("127.0.0.8:8091").await.expect("Failed to bind client socket");
    //register_user(client_socket_register,dos_address, "Client1","Client").await;
    //println!("Finished Registry");
    //query_online_users(client_socket_query,dos_address).await;
    let middleware_socket = UdpSocket::bind(&middleware_address).await.expect("Failed to bind middleware socket");

    tokio::spawn(middleware_task(middleware_socket));
    let dos_address_clone = dos_address.clone(); // Assuming `dos_address` is defined elsewhere
    let termination = Arc::new(Mutex::new(0));
    let termination_clone = Arc::clone(&termination);

    let (tx, _) = tokio::sync::broadcast::channel::<()>(1);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");

    let mut signal = signal(SignalKind::interrupt()).expect("Failed to create signal handler");
    tokio::spawn(async move {
        signal.recv().await;
        println!("Received termination signal");

        //let unregister_message = "UNREGISTER";
        //let dos_socket = UdpSocket::bind("127.0.0.8:9000").await.expect("Failed to bind socket");
        //dos_socket
        //    .send_to(unregister_message.as_bytes(), dos_address_clone)
        //    .await
        //    .expect("Failed to send unregister message");

        // Notify other tasks waiting for the signal
        //let _ = tx.send(());
        *termination_clone.lock().unwrap() = 1;
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Exit the application
        std::process::exit(0);
        //return;
    });

    while *termination.lock().unwrap() == 0 {
        println!("Press Enter to send a Request");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");
        
        if input.trim() == "" {
            for i in 1..2 {
                if (i % 90 == 0) {
                    sleep(Duration::from_millis(10)).await;
                }

                let request_message = Message::Request("Request from Client 2!".to_string());
                let serialized_request = serde_json::to_string(&request_message).expect("Failed to serialize request");

                client_socket
                    .send_to(serialized_request.as_bytes(), middleware_address)
                    .await
                    .expect("Failed to send request to middleware");
                println! ("Waiting for response.");

                // Receive response from the server (the first one)
                let mut client_buffer = [0; 1024];
                client_socket
                    .recv_from(&mut client_buffer)
                    .await
                    .expect("Failed to receive response from server");

                let response = String::from_utf8_lossy(&client_buffer);
                println!("Client received response from server: {}", response);
            }
        }
        if input.trim() == "Q"
        {
            //let unregister_message = "UNREGISTER";
            //let dos_socket = UdpSocket::bind("127.0.0.8:9001").await.expect("Failed to bind socket");
            //dos_socket
            //.send_to(unregister_message.as_bytes(), dos_address_clone)
            //.await
            //.expect("Failed to send unregister message");
            return;
        
        }
    }
        
    //middleware_task_handle.await.expect("Middleware task failed");
}