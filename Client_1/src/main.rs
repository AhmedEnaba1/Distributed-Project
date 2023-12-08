use async_std::net::UdpSocket;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fs, thread};
use steganography::decoder::Decoder;
use steganography::encoder::Encoder;
use steganography::util::{file_as_dynamic_image, save_image_buffer, str_to_bytes};
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

#[derive(Debug, Deserialize)]
struct User {
    address: String,
    name: String,
    user_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Request(String),
}

async fn middleware_task(mut middleware_socket: UdpSocket) {
    let server_addresses = [
        "127.0.0.1:8080",  "127.0.0.2:8080","127.0.0.3:8081"
    ];
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

    tokio::spawn(middleware_task(middleware_socket));
    let termination = Arc::new(Mutex::new(0));
    let termination_clone = Arc::clone(&termination);

    while *termination.lock().unwrap() == 0 {
        println!("Press Enter to send a Request");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let picture_data = fs::read("japan.png").expect("Failed to read image!");
        let frags = (picture_data.len() / 16384) + 1;
        let mut client_buffer = [0; 65536];

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
            println!("Waiting for response.");

            // Receive response from the server (the first one)

            client_socket
                .recv_from(&mut client_buffer)
                .await
                .expect("Failed to receive response from server");

            let response = String::from_utf8_lossy(&client_buffer);
            println!("Client received response from server: {}", response);
        }
        println!("Image Sent Succesfuly!\nRecieveing Encrypted image..");
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
                println!("Image saved successfully");
            }
        } else {
            println!("Failed to create image from byte stream");
        }
    }
}
