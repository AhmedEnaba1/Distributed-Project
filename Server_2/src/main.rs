use async_std::net::UdpSocket;
use serde::{de::Error, Deserialize, Serialize};
use serde_json::value::Index;
use serde_json::Result;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::time::Duration;
use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};
use steganography::encoder::Encoder;
use steganography::util::{file_as_dynamic_image, save_image_buffer, str_to_bytes};
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::Mutex as TokioMutex;
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize)]
struct frag {
    total_frags_number: usize,
    position: i16,
    packet: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct User {
    address: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Election(f32),
    Request(String),
    DOs(String),
}

fn getIP(socket_addr: SocketAddr) -> String {
    if let Some(ip) = socket_addr.ip().to_string().split(':').next() {
        return ip.to_string();
    }
    String::new()
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

async fn server_task(sys: &mut System) {
    let server_address: SocketAddr = format!("127.0.0.2:8080")
        .parse()
        .expect("Failed to parse server address");

    // Wrap the UdpSocket in Arc
    let client_socket = Arc::new(
        UdpSocket::bind(&server_address)
            .await
            .expect("Failed to bind server socket"),
    );

    let mut buffer = [0; 65536];
    let mut picture: Vec<u8> = Vec::new();
    let mut picture_frags = HashMap::<i16, Vec<u8>>::new();
    let mut serviced_client = 0;
    let mut current_client: SocketAddr = "0.0.0.0:0".parse().expect("Failed to parse");
    let mut leader = false;
    let mut registered_users: Vec<User> = Vec::new();

    println!("Server 2 is UP!");

    loop {
        sys.refresh_all();

        // Access CPU information
        let cpu = sys.global_cpu_info();
        let cpu_usage = cpu.cpu_usage();

        if let Ok((bytes_received, client_address)) = client_socket.recv_from(&mut buffer).await {
            let received_message: Message = serde_json::from_slice(&buffer[..bytes_received])
                .expect("Failed to deserialize message");

            match received_message {
                Message::DOs(query) => {
                    let mes: String = query;
                    if mes.starts_with("Register:") {
                        let parts: Vec<&str> = mes.trim().split(':').collect();
                        let username = parts[1].trim();
                        let user_address = client_address.ip().to_string();
                        let new_user = User {
                            address: user_address,
                            name: username.to_string(),
                        };
                        registered_users.push(new_user);
                        println!("User ({}) registered", username);
                        let response = "User registered";
                        client_socket
                            .send_to(response.as_bytes(), client_address)
                            .await
                            .expect("Failed to send response");
                        continue;
                    } else if mes.starts_with("Remove") {
                        let user_address = client_address.ip().to_string();
                        if let Some(index) = registered_users
                            .iter()
                            .position(|user| user.address == user_address)
                        {
                            println!("User ({}) went offline.", registered_users[index].name);
                            registered_users.remove(index);
                        }
                        let response = "User";
                        client_socket
                            .send_to(response.as_bytes(), client_address)
                            .await
                            .expect("Failed to send response");
                        continue;
                    } else {
                        println!("Server 2 received request!, from {}", client_address);
                        println!("Server 2 is starting an election.");
                        leader = start_election(cpu_usage).await;

                        if leader {
                            let reg_users = serde_json::to_string(&registered_users)
                                .expect("Failed to serialize active users to JSON");
                            client_socket
                                .send_to(reg_users.as_bytes(), client_address)
                                .await
                                .expect("Failed to send active users list");
                        }
                        continue;
                    }
                }
                Message::Request(request) => {
                    // Start a new election
                    if serviced_client == 0 {
                        println!("Server 2 received request!, from {}", client_address);
                        println!("Server 2 is starting an election.");
                        leader = start_election(cpu_usage).await;
                        if leader {
                            current_client = client_address;
                            println!("Servcing: {}", current_client);
                        }
                    }
                    if leader || (current_client == client_address) {
                        let frag: frag = serde_json::from_str(&request).unwrap();
                        if frag.position != -1 {
                            serviced_client = 1;
                            picture_frags.insert(frag.position, frag.packet);

                            let response_message = Message::Request(format!("Ack from Server 2"));
                            client_socket
                                .send_to(
                                    &serde_json::to_string(&response_message).unwrap().as_bytes(),
                                    client_address, // Replace with the actual client address
                                )
                                .await
                                .expect("Failed to send response to client");
                        } else {
                            let response_message = Message::Request(format!("Ack from Server 2"));
                            client_socket
                                .send_to(
                                    &serde_json::to_string(&response_message).unwrap().as_bytes(),
                                    client_address, // Replace with the actual client address
                                )
                                .await
                                .expect("Failed to send response to client");
                            picture_frags
                                .insert(frag.total_frags_number.try_into().unwrap(), frag.packet);

                            let picture_clone: BTreeMap<_, _> =
                                picture_frags.clone().into_iter().collect();

                            for (_key, value) in picture_clone {
                                picture.extend_from_slice(&value);
                            }
                            remove_trailing_zeros(&mut picture);

                            // if let Ok(picture_recv) = image::load_from_memory(&picture) {
                            //     if let Err(_err) = picture_recv.save("j_out.png") {
                            //         eprintln!("Failed to save image");
                            //     } else {
                            let mut _ack_buffer = [0; 1024];

                            let image_string = base64::encode(picture.clone());
                            let payload = str_to_bytes(&image_string);
                            let destination_image = file_as_dynamic_image("Mask.png".to_string());
                            let enc = Encoder::new(payload, destination_image);
                            let result = enc.encode_alpha();
                            save_image_buffer(result, "encrypted.png".to_string());
                            println!("Recieved Image successfully!");

                            picture.clear();

                            _ack_buffer = [0; 1024];

                            let picture_data =
                                fs::read("encrypted.png").expect("Failed to read image!");
                            let frags = (picture_data.len() / 16384) + 1;

                            println!("Sending Picture to client...");

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
                                    .send_to(serialized_request.as_bytes(), client_address)
                                    .await
                                    .expect("Failed to send request to client");

                                // Receive response from the server (the first one)
                                let mut message_buffer = [0; 65536];
                                client_socket
                                    .recv_from(&mut message_buffer)
                                    .await
                                    .expect("Failed to receive response from client");

                                let _response = String::from_utf8_lossy(&message_buffer);
                                //println!("Server received response from client: {}", response);
                            }
                            println!("Sent Encrypted Image!");
                            //}
                            picture.clear();
                            // } else {
                            //     println!("Failed to create image!");
                            // }
                            picture_frags.clear();
                            serviced_client = 0;
                            current_client = "0.0.0.0:0".parse().expect("Failed to parse");
                        }
                        leader = false;
                    }
                }
                _ => {}
            }

            // Clear the buffer for the next message
            buffer = [0; 65536];
        }
    }
}

async fn start_election(cpu_usage: f32) -> bool {
    // Wrap the UdpSocket in Arc
    let socket2 = Arc::new(
        UdpSocket::bind("127.0.0.2:2112")
            .await
            .expect("Failed to bind server socket"),
    );

    // Wrap the UdpSocket in Arc
    let socket3 = Arc::new(
        UdpSocket::bind("127.0.0.2:2113")
            .await
            .expect("Failed to bind server socket"),
    );

    // Broadcast Election messages to servers with higher IDs
    let election_message = Message::Election(cpu_usage);

    // Use Arc to share ownership without requiring Clone
    socket2
        .send_to(
            &serde_json::to_string(&election_message).unwrap().as_bytes(),
            "127.0.0.1:2112", // Replace with actual addresses of other servers
        )
        .await
        .expect("Failed to send Election message");

    socket3
        .send_to(
            &serde_json::to_string(&election_message).unwrap().as_bytes(),
            "127.0.0.3:2113", // Replace with actual addresses of other servers
        )
        .await
        .expect("Failed to send Election message");

    // Set a timeout for receiving messages
    let timeout = Duration::from_millis(500);

    // Update the server IDs
    let cpu_usage2 = receive_election_message(Arc::clone(&socket2), timeout)
        .await
        .unwrap_or(100.0);
    let cpu_usage3 = receive_election_message(Arc::clone(&socket3), timeout)
        .await
        .unwrap_or(100.0);

    println!("IDs: {},{},{}", cpu_usage, cpu_usage2, cpu_usage3);

    let mut leader = false;

    // Check if the current server is the Leader or not
    if cpu_usage < cpu_usage2 && cpu_usage < cpu_usage3 {
        leader = true;
        println!("This Server is the Coordinator!");
    }

    leader
}

async fn receive_election_message(socket: Arc<UdpSocket>, timeout: Duration) -> Result<f32> {
    let mut buffer = [0; 1024];
    let mut server_id: f32 = 100.0;

    // Use Arc to share ownership without requiring Clone
    let result = tokio::time::timeout(timeout, async {
        if let Ok((bytes_received, _)) = socket.recv_from(&mut buffer).await {
            let received_message: Message = serde_json::from_slice(&buffer[..bytes_received])
                .expect("Failed to deserialize message");

            if let Message::Election(id) = received_message {
                server_id = id;
            }
        }
    })
    .await;

    result
        .map(|_| server_id)
        .map_err(|_| serde_json::Error::custom("Timeout occurred"))
}

#[tokio::main]
async fn main() {
    let mut sys = System::new();

    let task = server_task(&mut sys);
    let _ = tokio::join!(task);
}
