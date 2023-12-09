use async_std::net::UdpSocket;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use steganography::decoder::Decoder;
use steganography::encoder::Encoder;
use steganography::util::{file_as_dynamic_image, save_image_buffer, str_to_bytes};
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize)]
struct frag {
    total_frags_number: usize,
    position: i16,
    packet: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct User {
    address: String,
    name: String,
    user_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Request(String),
    DOs(String),
}

async fn query_online_users(client_socket: UdpSocket) {
    // Send a query message to request the list of online users
    let server_addresses = ["127.0.0.2:8080", "127.0.0.1:8080", "127.0.0.4:8080"];
    let query_message = Message::DOs(format!("Query:"));
    let query_json = serde_json::to_string(&query_message).expect("Failed to serialize message");

    for server_address in &server_addresses {
        client_socket
            .send_to(query_json.as_bytes(), server_address)
            .await
            .expect("Failed to send query request");
    }
    let mut response_buffer = [0; 65536];
    let (bytes_received, _middleware_address) = client_socket
        .recv_from(&mut response_buffer)
        .await
        .expect("Failed to receive response");
    let response = String::from_utf8_lossy(&response_buffer[..bytes_received]);
    println!("Online users: {}", response);
}

async fn register_user(client_socket: UdpSocket, username: &str) {
    let server_addresses = [
        "127.0.0.2:8080",
        "127.0.0.1:8080", /*, "127.0.0.4:8080"*/
    ];
    let registration_message = Message::DOs(format!("Register:{}", username));
    let message_json =
        serde_json::to_string(&registration_message).expect("Failed to serialize message");

    for server_address in &server_addresses {
        client_socket
            .send_to(message_json.as_bytes(), server_address)
            .await
            .expect("Failed to send registration request");
    }
}

async fn middleware_task(mut middleware_socket: UdpSocket) {
    let server_addresses = ["127.0.0.1:8080", "127.0.0.2:8080", "127.0.0.3:8080"];
    let mut buffer = [0; 65536];
    let mut ack_buffer = [0; 1024];

    let mut replier_server = "".to_string();

    while let Ok((bytes_received, client_address)) = middleware_socket.recv_from(&mut buffer).await
    {
        if client_address.ip().to_string() == "127.0.0.8" {
            let received_message: Message = serde_json::from_slice(&buffer[..bytes_received])
                .expect("Failed to deserialize message");
            let request = match received_message {
                Message::Request(request_string) => request_string,
                Message::DOs(_) => todo!(),
            };
            let frag: frag = serde_json::from_str(&request).unwrap();
            if (frag.position == 0) {
                let se = serde_json::to_string(&frag).unwrap();
                let request_message = Message::Request(se);
                let serialized_request =
                    serde_json::to_string(&request_message).expect("Failed to serialize request");

                for server_address in &server_addresses {
                    let server_address: SocketAddr = server_address
                        .parse()
                        .expect("Failed to parse server address");

                    middleware_socket
                        .send_to(serialized_request.as_bytes(), &server_address)
                        .await
                        .expect("Failed to send data to server");
                }
            } else {
                let se = serde_json::to_string(&frag).unwrap();
                let request_message = Message::Request(se);
                let serialized_request =
                    serde_json::to_string(&request_message).expect("Failed to serialize request");
                let server_address: SocketAddr = replier_server
                    .parse()
                    .expect("Failed to parse server address");
                middleware_socket
                    .send_to(serialized_request.as_bytes(), &server_address)
                    .await
                    .expect("Failed to send data to server");
            }

            if let Ok((_, replier)) = middleware_socket.recv_from(&mut ack_buffer).await {
                replier_server = replier.to_string();
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
            let my_client_address = "127.0.0.8:3411";
            middleware_socket
                .send_to(&buffer[0..bytes_received], my_client_address)
                .await
                .expect("Failed to send acknowledgment to client");
            middleware_socket
                .recv_from(&mut ack_buffer)
                .await
                .expect("Failed");
            middleware_socket
                .send_to(&buffer[0..bytes_received], client_address)
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

fn remove_trailing_zeros(data: &mut Vec<u8>) {
    // Find the index of the last element that is not zero
    let last_non_zero_index = data.iter().rev().position(|&x| x != 0);

    if let Some(index) = last_non_zero_index {
        // Calculate the index in the original order
        let index = data.len() - index - 1;

        // Truncate the vector to remove trailing zeros
        data.truncate(index + 1);
    } else {
        // The vector is either empty or contains only zeros
        data.clear();
    }
}

#[tokio::main]
async fn main() {
    let middleware_address: SocketAddr = "127.0.0.8:12345"
        .parse()
        .expect("Failed to parse middleware address");

    let middleware_socket = UdpSocket::bind(&middleware_address)
        .await
        .expect("Failed to bind middleware socket");

    let client_socket = UdpSocket::bind("127.0.0.8:3411")
        .await
        .expect("Failed to bind client socket");

    let DOS_add_socket = UdpSocket::bind("127.0.0.8:8090")
        .await
        .expect("Failed to bind client socket");

    tokio::spawn(middleware_task(middleware_socket));
    let termination = Arc::new(Mutex::new(0));
    let termination_clone = Arc::clone(&termination);
    let mut signal = signal(SignalKind::interrupt()).expect("Failed to create signal handler");
    let username = "Client1".to_string();

    register_user(DOS_add_socket, &username).await;

    tokio::spawn(async move {
        signal.recv().await;
        println!("\nReceived termination signal");
        let server_addresses = [
            "127.0.0.2:8080",
            "127.0.0.1:8080", /* , "127.0.0.4:8080"*/
        ];

        let remove_mes = Message::DOs(format!("Remove"));
        let remove_json = serde_json::to_string(&remove_mes).expect("Failed to serialize message");
        let dos_socket = UdpSocket::bind("127.0.0.8:9000")
            .await
            .expect("Failed to bind socket");
        for server_address in &server_addresses {
            dos_socket
                .send_to(remove_json.as_bytes(), server_address)
                .await
                .expect("Failed to send unregister message");
        }
        *termination_clone.lock().unwrap() = 1;

        // Exit the application
        std::process::exit(0);
    });
    println!("Welcome to P2P app!\nPlease enter an option:");
    while *termination.lock().unwrap() == 0 {
        println!("1. View a Picture");
        println!("2. View Online Users");
        println!("3. Request image From Client");
        println!("4. Request more Views");
        println!("5. Control Views");
        println!("6. Go Online/Offline");
        println!("\nSelected option: ");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        if input.trim() == "2" {
            let client_socket_query = UdpSocket::bind("127.0.0.8:8091")
                .await
                .expect("Failed to bind client socket");
            query_online_users(client_socket_query).await;
        } else {
            let picture_data = fs::read("japan.png").expect("Failed to read image!");
            let frags = (picture_data.len() / 16384) + 1;
            let mut client_buffer = [0; 65536];

            println!("Sending Image for encryption...");

            for (index, piece) in picture_data.chunks(16384).enumerate() {
                let end = index == frags - 1;
                let frag = frag {
                    packet: piece.to_vec(),
                    position: if end { -1 } else { index.try_into().unwrap() },
                    total_frags_number: frags,
                };

                let se = serde_json::to_string(&frag).unwrap();
                let request_message = Message::Request(se);
                let serialized_request =
                    serde_json::to_string(&request_message).expect("Failed to serialize request");

                client_socket
                    .send_to(serialized_request.as_bytes(), middleware_address)
                    .await
                    .expect("Failed to send request to middleware");

                // Receive response from the server (the first one)

                client_socket
                    .recv_from(&mut client_buffer)
                    .await
                    .expect("Failed to receive response from server");

                let _response = String::from_utf8_lossy(&client_buffer);
            }
            println!("Image Sent Succesfuly!\nRecieveing Image..");
            let mut encrypted_image_data: Vec<u8> = Vec::new();
            let mut image_chunks = HashMap::<i16, Vec<u8>>::new();
            let mut j = 0;
            let mut total = frags;
            while (j < total) {
                if let Ok((bytes_received, client_address)) =
                    client_socket.recv_from(&mut client_buffer).await
                {
                    let received_message: Message =
                        serde_json::from_slice(&client_buffer[..bytes_received])
                            .expect("Failed to deserialize message");
                    let recieved = match received_message {
                        Message::Request(request_string) => request_string,
                        Message::DOs(_) => todo!(),
                    };
                    let frag: frag = serde_json::from_str(&recieved).unwrap();
                    total = frag.total_frags_number;
                    if j == total - 1 {
                        image_chunks.insert(total.try_into().unwrap(), frag.packet);
                    } else {
                        image_chunks.insert(frag.position, frag.packet);
                    }
                    j += 1;
                    let response_message = Message::Request(format!("Ack from Server 2"));
                    client_socket
                        .send_to(
                            &serde_json::to_string(&response_message).unwrap().as_bytes(),
                            client_address, // Replace with the actual client address
                        )
                        .await
                        .expect("Failed to send response to client");
                    client_buffer = [0; 65536]
                }
            }
            let image_chunks_cloned: BTreeMap<_, _> = image_chunks.clone().into_iter().collect();

            for (_key, value) in image_chunks_cloned {
                // println!("Key: {}, Value: {:?}", key, value);
                encrypted_image_data.extend_from_slice(&value);
            }

            remove_trailing_zeros(&mut encrypted_image_data);

            if let Ok(encrypted_image) = image::load_from_memory(&encrypted_image_data) {
                if let Err(err) = encrypted_image.save("encrypted.png") {
                    eprintln!("Failed to save the image: {}", err);
                } else {
                    println!("Image saved successfully\n");
                }
            } else {
                println!("Failed to create image from byte stream");
            }
        }
    }
}
