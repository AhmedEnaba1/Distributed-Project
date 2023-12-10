use async_std::net::UdpSocket;
use chrono::Local;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Cursor};
use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use steganography::decoder::Decoder;
use steganography::encoder::Encoder;
use steganography::util::*;
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{sleep, timeout};

static mut status: bool = true;

#[derive(Debug, Serialize, Deserialize)]
struct frag {
    total_frags_number: usize,
    position: i16,
    packet: Vec<u8>,
    views: u8,
}

#[derive(Debug, Deserialize)]
struct User {
    address: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Request(String),
    DOs(String),
}

async fn p2p(peer_socket: UdpSocket) {
    let mut buffer = [0; 65536];
    let mut ack_buffer = [0; 65536];
    let mut serviced_clients: Vec<String> = Vec::new();
    while let Ok((_bytes_received, client_address)) = peer_socket.recv_from(&mut buffer).await {
        while unsafe { status == false } {}
        let message = String::from_utf8_lossy(&buffer[.._bytes_received]);

        if message.starts_with("New") {
            println!("Users:");
            for (index, user) in serviced_clients.iter().enumerate() {
                println!("{}: {}", index + 1, user);
            }
            let mut user_input = String::new();
            let mut desired_user = "".to_string();
            loop {
                println!("Choose the user:");
                user_input.clear();

                io::stdin()
                    .read_line(&mut user_input)
                    .expect("Failed to read line");

                match user_input.trim().parse::<usize>() {
                    Ok(index) if index > 0 && index <= serviced_clients.len() => {
                        // User entered a valid number
                        let selected_user = &serviced_clients[index - 1];
                        println!("You selected: {}", selected_user);
                        desired_user = selected_user.to_string();
                        break;
                    }
                    _ => {
                        // Invalid input, ask the user again
                        println!("Invalid input. Please enter a valid number.");
                    }
                }
            }
            loop {
                println!("Enter a number of views:");
                user_input.clear();

                io::stdin()
                    .read_line(&mut user_input)
                    .expect("Failed to read line");

                match user_input.trim().parse::<usize>() {
                    Ok(views) => {
                        // Valid usize entered by the user
                        let v: &[u8] = &views.to_be_bytes();
                        let parts: Vec<&str> = desired_user.split(':').collect();
                        // Take the first part (IP address)
                        let mut user_s = "".to_string();
                        if let Some(ip) = parts.get(0) {
                            // Create a new string with the IP and the desired port
                            desired_user = format!("{}:8085", ip);
                            user_s = desired_user;
                        }
                        peer_socket.send_to("Sig".as_bytes(), user_s).await;
                        let (_, car) = peer_socket
                            .recv_from(&mut ack_buffer)
                            .await
                            .expect("Couldn't recieve index");
                        peer_socket.send_to(v, car).await;
                        break;
                    }
                    Err(_) => {
                        // Invalid input, ask the user again
                        println!("Invalid input. Please enter a valid number.");
                    }
                }
            }
            peer_socket.send_to("Sig".as_bytes(), client_address).await;
            continue;
        }

        if message.starts_with("Sig") {
            peer_socket.send_to("buf".as_bytes(), client_address).await;
            peer_socket.recv_from(&mut buffer).await;
            if let Ok(entries) = fs::read_dir("RCV/") {
                // Filter files that are PNG and contain the client address in their name
                let png_files: Vec<String> = entries
                    .filter_map(|entry| {
                        if let Ok(entry) = entry {
                            if let Some(file_name) = entry.file_name().to_str() {
                                if file_name.ends_with(".png")
                                    && file_name.contains(&client_address.to_string())
                                {
                                    return Some(file_name.to_string());
                                }
                            }
                        }
                        None
                    })
                    .collect();

                // Print the found PNG files
                if !png_files.is_empty() {
                    println!("PNG files with client address in the name:");
                    for file_name in png_files {
                        let encoded_image =
                            file_as_image_buffer(format!("RCV/{}", file_name.trim_matches('"')));
                        let dec = Decoder::new(encoded_image);
                        let out_buffer = dec.decode_alpha();
                        let clean_buffer: Vec<u8> =
                            out_buffer.into_iter().filter(|b| *b != 0xff_u8).collect();
                        let message = bytes_to_str(clean_buffer.as_slice());

                        let mut decoded_image_data = base64::decode(message).unwrap_or_else(|e| {
                            eprintln!("Error decoding base64: {}", e);
                            Vec::new()
                        });
                        decoded_image_data[0] = decoded_image_data[0] + buffer[7];
                        println!("New Views:{}", decoded_image_data[0]);
                        if let Ok(encrypted_image) =
                            image::load_from_memory(&decoded_image_data[1..])
                        {
                            if let Err(err) = encrypted_image.save("RCV/decoded.png") {
                                eprintln!("Failed to save the image: {}", err);
                            } else {
                                println!("Image saved successfully");
                            }
                        } else {
                            println!("Failed to create image from byte stream");
                        }

                        if let Ok(_image) = image::open("RCV/decoded.png") {
                            println!("Viewing image: decoded.png");
                            if let Ok(_) = open::that("RCV/decoded.png") {
                                println!("Opened Image");
                            } else {
                                println!("Failed to Open");
                            }
                        } else {
                            println!("Failed to open image");
                        }

                        let image_string = base64::encode(decoded_image_data.clone());
                        let payload = str_to_bytes(&image_string);
                        let destination_image = file_as_dynamic_image("encrypt.png".to_string());
                        let enc = Encoder::new(payload, destination_image);
                        let result = enc.encode_alpha();
                        save_image_buffer(result, format!("RCV/{}", file_name.trim_matches('"')));
                    }
                } else {
                    println!("No matching PNG files found.");
                }

                continue;
            }
        }

        let png_files: Vec<String> = fs::read_dir("Own/")
            .expect("Failed to read directory")
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    if e.path().extension().and_then(|e| e.to_str()) == Some("png") {
                        Some(e.file_name().to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
            })
            .collect();
        let png_file_legnth: &[u8] = &png_files.len().to_be_bytes();
        peer_socket
            .send_to(png_file_legnth, client_address)
            .await
            .expect("Failed to send data to server");
        let mut iterate = 0;
        while iterate < png_files.len() {
            println!("{}", &png_files[iterate]);
            let img = image::open(format!("Own/{}", &png_files[iterate].trim_matches('"')))
                .expect("Failed to open image");
            let low_res = img.resize(512, 512, image::imageops::FilterType::Nearest);

            // let image_data = fs::read(&png_files[iterate]).expect("Failed to read the image file");
            let mut image_data = Vec::new();
            low_res
                .write_to(
                    &mut Cursor::new(&mut image_data),
                    image::ImageOutputFormat::Png,
                )
                .expect("Failed to write image to byte steam");

            let packet_number = (image_data.len() / 16384) + 1;
            println!("Packets:{}", packet_number);
            for (index, piece) in image_data.chunks(16384).enumerate() {
                let end = index == packet_number - 1;
                let frag = frag {
                    views: unsafe { 4 },
                    total_frags_number: packet_number,
                    position: if end { -1 } else { index.try_into().unwrap() },
                    packet: { piece.to_vec() },
                };

                let serialized_request = serde_json::to_string(&frag).unwrap();

                peer_socket
                    .send_to(&serialized_request.as_bytes(), client_address)
                    .await
                    .expect("Failed to send piece to middleware");
                let (num_bytes_received, _) = peer_socket
                    .recv_from(&mut ack_buffer)
                    .await
                    .expect("Failed to receive acknowledgement from server");
            }
            iterate += 1;
        }
        let png_files_json =
            serde_json::to_vec(&png_files).expect("Failed to serialize png_files to JSON");
        peer_socket
            .send_to(&png_files_json, client_address)
            .await
            .expect("Failed to send piece to client");
        let (num_bytes_received, cadd) = peer_socket
            .recv_from(&mut buffer)
            .await
            .expect("Failed to receive acknowledgement from server");
        serviced_clients.push(cadd.to_string());
        let filename = format!(
            "Own/{}",
            String::from_utf8_lossy(&buffer[..num_bytes_received]).to_string()
        );
        let (num_bytes_received, _) = peer_socket
            .recv_from(&mut buffer)
            .await
            .expect("Failed to receive acknowledgement from server");
        let requested_views = u8::from_le_bytes([buffer[7]]);
        println!("RVs:{}", requested_views);

        //Start Encryption
        let image_data = fs::read(filename).expect("Failed to read the image file");
        let middleware_address = "127.0.0.11:12345"; // Replace with the actual middleware address and port
                                                     //sleep(Duration::from_millis(5000)).await;

        let packet_number = (image_data.len() / 16384) + 1;
        for (index, piece) in image_data.chunks(16384).enumerate() {
            let end = index == packet_number - 1;
            let frag = frag {
                views: requested_views,
                total_frags_number: packet_number,
                position: if end { -1 } else { index.try_into().unwrap() },
                packet: { piece.to_vec() },
            };
            let se = serde_json::to_string(&frag).unwrap();
            let request_message = Message::Request(se);
            let serialized_request =
                serde_json::to_string(&request_message).expect("Failed to serialize request");

            peer_socket
                .send_to(&serialized_request.as_bytes(), middleware_address)
                .await
                .expect("Failed to send piece to middleware");

            let (num_bytes_received, _) = peer_socket
                .recv_from(&mut buffer)
                .await
                .expect("Failed to receive acknowledgement from server");
        }

        let mut encrypted_image_data: Vec<u8> = Vec::new();
        let mut image_chunks = HashMap::<i16, Vec<u8>>::new();
        let mut j = 0;
        let mut enc_total = packet_number;

        while j < enc_total {
            if let Ok((_bytes_received, _client_address)) = peer_socket.recv_from(&mut buffer).await
            {
                let received_message: Message = serde_json::from_slice(&buffer[.._bytes_received])
                    .expect("Failed to deserialize message");
                let recieved = match received_message {
                    Message::Request(request_string) => request_string,
                    Message::DOs(_) => todo!(),
                };
                let deserialized: frag = serde_json::from_str(&recieved).unwrap();
                enc_total = deserialized.total_frags_number;
                buffer = [0; 65536];

                if j == enc_total - 1 {
                    image_chunks.insert(enc_total.try_into().unwrap(), deserialized.packet);
                } else {
                    image_chunks.insert(deserialized.position, deserialized.packet);
                }
                j += 1;
                peer_socket
                    .send_to("Ack".as_bytes(), _client_address)
                    .await
                    .expect("Failed to send");
            }
        }
        let image_chunks_cloned: BTreeMap<_, _> = image_chunks.clone().into_iter().collect();

        for (_key, value) in image_chunks_cloned {
            // println!("Key: {}, Value: {:?}", key, value);
            encrypted_image_data.extend_from_slice(&value);
        }

        remove_trailing_zeros(&mut encrypted_image_data);

        //End of Encryption
        let frags = (encrypted_image_data.len() / 16384) + 1;
        for (index, piece) in encrypted_image_data.chunks(16384).enumerate() {
            let end = index == frags - 1;
            let frag = frag {
                views: requested_views,
                total_frags_number: frags,
                position: if end { -1 } else { index.try_into().unwrap() },
                packet: { piece.to_vec() },
            };
            let serialized_request = serde_json::to_string(&frag).unwrap();

            peer_socket
                .send_to(&serialized_request.as_bytes(), client_address)
                .await
                .expect("Failed to send piece to middleware");

            let (num_bytes_received, _) = peer_socket
                .recv_from(&mut buffer)
                .await
                .expect("Failed to receive acknowledgement from server");
        }
    }
}

async fn query_online_users(client_socket: UdpSocket, username: String) {
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
    let users: Vec<User> = serde_json::from_slice(&response_buffer[..bytes_received])
        .expect("Failed to deserialize users");
    let filtered_users: Vec<_> = users.iter().filter(|user| user.name != username).collect();

    // Display enumerated names of filtered users
    for (index, user) in filtered_users.iter().enumerate() {
        println!("{}: {}", index + 1, user.name);
    }
    println!("\nWhat else would like to do? Please enter an option:")
}

async fn connect(client_socket: UdpSocket, username: String) -> std::string::String {
    // Send a query message to request the list of online users
    let server_addresses = ["127.0.0.2:8080", "127.0.0.1:8080", "127.0.0.4:8080"];
    let mut peer_address = "".to_string();
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
    let users: Vec<User> = serde_json::from_slice(&response_buffer[..bytes_received])
        .expect("Failed to deserialize users");
    let filtered_users: Vec<_> = users.iter().filter(|user| user.name != username).collect();

    // Display enumerated names of filtered users
    for (index, user) in filtered_users.iter().enumerate() {
        println!("{}: {}", index + 1, user.name);
    }

    // Ask the user to choose an option
    println!("Choose a user by entering its number:");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let chosen_index: usize = input.trim().parse().expect("Invalid input");

    // Check if the index is valid
    if chosen_index > 0 && chosen_index <= filtered_users.len() {
        let chosen_user = &filtered_users[chosen_index - 1];
        peer_address = format!("{}:8085", chosen_user.address);
    } else {
        println!("Invalid index. Please provide a valid number.");
    }

    return peer_address;
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
        if client_address.ip().to_string() == "127.0.0.11" {
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
            let my_client_address = "127.0.0.11:3411";
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
    let middleware_address: SocketAddr = "127.0.0.11:12345"
        .parse()
        .expect("Failed to parse middleware address");

    let middleware_socket = UdpSocket::bind(&middleware_address)
        .await
        .expect("Failed to bind middleware socket");

    let client_socket = UdpSocket::bind("127.0.0.11:3411")
        .await
        .expect("Failed to bind client socket");

    let peer_socket = UdpSocket::bind("127.0.0.11:8085")
        .await
        .expect("Failed to bind middleware socket");

    let DOS_add_socket = UdpSocket::bind("127.0.0.11:8090")
        .await
        .expect("Failed to bind client socket");

    tokio::spawn(middleware_task(middleware_socket));
    tokio::spawn(p2p(peer_socket));

    let termination = Arc::new(Mutex::new(0));
    let termination_clone = Arc::clone(&termination);
    let mut signal = signal(SignalKind::interrupt()).expect("Failed to create signal handler");
    let username = "Client2".to_string();

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
        let dos_socket = UdpSocket::bind("127.0.0.11:9000")
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
        println!("4. Control Views");
        println!("5. Log OFF/ Login");
        println!("\nSelected option: ");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        if input.trim() == "2" {
            let client_socket_query = UdpSocket::bind("127.0.0.11:8091")
                .await
                .expect("Failed to bind client socket");
            query_online_users(client_socket_query, "Client2".to_string()).await;
        } else if input.trim() == "1" {
            if let Ok(metadata) = fs::metadata("RCV/decoded.png") {
                if metadata.is_file() {
                    if let Err(err) = fs::remove_file("RCV/decoded.png") {
                        eprintln!("Error deleting file: {}", err);
                    } else {
                        println!("File deleted successfully");
                    }
                } else {
                    println!("The path exists, but it is not a file.");
                }
            } else {
                println!("The file does not exist.\n");
                continue;
            }
            let png_files: Vec<_> = fs::read_dir("RCV/")
                .expect("Failed to read directory")
                .filter_map(|entry| {
                    entry.ok().and_then(|e| {
                        if e.path().extension().and_then(|e| e.to_str()) == Some("png") {
                            Some(e.file_name())
                        } else {
                            None
                        }
                    })
                })
                .collect();

            // Display enumerated PNG file names
            for (index, file_name) in png_files.iter().enumerate() {
                println!("{}: {}", index + 1, file_name.to_string_lossy());
            }

            // Ask the user to input a number
            println!("Type the number of the image you want to view:");
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");

            // Parse the user input as an index
            let index: usize = input.trim().parse().expect("Invalid input");

            // Check if the index is valid
            if index > 0 && index <= png_files.len() {
                // Load and display the selected image
                let selected_file = &png_files[index - 1];
                let path = format!("RCV/{}", selected_file.to_string_lossy().trim_matches('"'));
                let image_path = Path::new(&path);
                println!("{},{}", path, image_path.to_string_lossy());

                let encoded_image = file_as_image_buffer(format!(
                    "RCV/{}",
                    selected_file.to_string_lossy().trim_matches('"')
                ));
                let dec = Decoder::new(encoded_image);
                let out_buffer = dec.decode_alpha();
                let clean_buffer: Vec<u8> =
                    out_buffer.into_iter().filter(|b| *b != 0xff_u8).collect();
                let message = bytes_to_str(clean_buffer.as_slice());

                let mut decoded_image_data = base64::decode(message).unwrap_or_else(|e| {
                    eprintln!("Error decoding base64: {}", e);
                    Vec::new()
                });
                println!("{}", decoded_image_data[0]);
                if (decoded_image_data[0] == 0) {
                    println!("Image views have finished. Please send the client another request to increase them!");
                }
                if (decoded_image_data[0] > 0) {
                    decoded_image_data[0] = decoded_image_data[0] - 1;

                    if let Ok(encrypted_image) = image::load_from_memory(&decoded_image_data[1..]) {
                        let (width, height) = encrypted_image.dimensions();

                        if let Err(err) = encrypted_image.save("RCV/decoded.png") {
                            eprintln!("Failed to save the image: {}", err);
                        } else {
                            println!("Image saved successfully");
                        }
                    } else {
                        println!("Failed to create image from byte stream");
                    }

                    if let Ok(_image) = image::open("RCV/decoded.png") {
                        println!("Viewing image: decoded.png");
                        let stats = Command::new("xdg-open")
                            .arg("RCV/decoded.png")
                            .status()
                            .expect("Failed to open the image.");

                        // Check the exit status of the command
                        if !stats.success() {
                            eprintln!("Error opening the image.");
                        }
                    } else {
                        println!("Failed to open image");
                    }

                    let image_string = base64::encode(decoded_image_data.clone());
                    let payload = str_to_bytes(&image_string);
                    let destination_image = file_as_dynamic_image("encrypt.png".to_string());
                    let enc = Encoder::new(payload, destination_image);
                    let result = enc.encode_alpha();
                    save_image_buffer(
                        result,
                        format!("RCV/{}", selected_file.to_string_lossy().trim_matches('"')),
                    );
                }
            } else {
                println!("Invalid index. Please provide a valid number.");
            }
        } else if input.trim() == "3" {
            let client_socket_query = UdpSocket::bind("127.0.0.11:8091")
                .await
                .expect("Failed to bind client socket");

            let connected_address = connect(client_socket_query, "Client2".to_string()).await;
            let mut request_buffer = [0; 65536];
            let mut small_buffer = [0; 8];
            client_socket
                .send_to("I want to request a picture".as_bytes(), connected_address)
                .await
                .expect("Failed to send");
            let (_, client_sender_address) =
                client_socket.recv_from(&mut small_buffer).await.expect("");
            let mut num_pictures = i32::from_be_bytes([
                small_buffer[4],
                small_buffer[5],
                small_buffer[6],
                small_buffer[7],
            ]);
            println!("Num Pics:{}", num_pictures);
            let mut i = 0;
            let mut enc_total: usize = 1000;
            let mut encrypted_image_data: Vec<u8> = Vec::new();
            let mut image_chunks = HashMap::<i16, Vec<u8>>::new();
            let mut j = 0;
            let mut image_vector: Vec<String> = Vec::new();
            while i < num_pictures {
                while j < enc_total {
                    if let Ok((_bytes_received, _client_address)) =
                        client_socket.recv_from(&mut request_buffer).await
                    {
                        let packet_string =
                            String::from_utf8_lossy(&request_buffer[0.._bytes_received]);
                        let deserialized: frag = serde_json::from_str(&packet_string).unwrap();
                        enc_total = deserialized.total_frags_number;
                        request_buffer = [0; 65536];
                        if j == enc_total - 1 {
                            image_chunks.insert(enc_total.try_into().unwrap(), deserialized.packet);
                        } else {
                            image_chunks.insert(deserialized.position, deserialized.packet);
                        }
                        j += 1;
                        client_socket
                            .send_to("Ack".as_bytes(), _client_address)
                            .await
                            .expect("Failed to send");
                    }
                }

                j = 0;
                i += 1;

                let image_chunks_cloned: BTreeMap<_, _> =
                    image_chunks.clone().into_iter().collect();
                image_chunks.clear();

                for (_key, value) in image_chunks_cloned {
                    encrypted_image_data.extend_from_slice(&value);
                }

                remove_trailing_zeros(&mut encrypted_image_data);

                if let Ok(encrypted_image) = image::load_from_memory(&encrypted_image_data) {
                    if let Err(err) = encrypted_image.save(format!("RCV/encrypted{}.png", i)) {
                        eprintln!("Failed to save the image: {}", err);
                    } else {
                        println!("Image saved successfully");
                    }
                } else {
                    println!("Failed to create image from byte stream");
                }
                if let Ok(_image) = image::open(format!("RCV/encrypted{}.png", i)) {
                    let stats = Command::new("xdg-open")
                        .arg(format!("RCV/encrypted{}.png", i))
                        .status()
                        .expect("Failed to open the image.");

                    // Check the exit status of the command
                    if !stats.success() {
                        eprintln!("Error opening the image.");
                    }
                } else {
                    println!("Failed to open image:");
                }
                image_vector.push(format!("RCV/encrypted{}.png", i));
                encrypted_image_data.clear();
            }
            let (num_bytes_received, client_address) = client_socket
                .recv_from(&mut request_buffer)
                .await
                .expect("Failed to receive data");

            // Deserialize the JSON into a vector of strings
            let png_files: Vec<String> =
                serde_json::from_slice(&request_buffer[..num_bytes_received])
                    .expect("Failed to deserialize JSON");

            // Display the list of strings to the user and let them choose
            println!("Choose a Picture:");
            for (index, png_file) in png_files.iter().enumerate() {
                println!("{}. {}", index + 1, png_file);
            }

            // Get user input for the chosen index
            let mut user_input = String::new();
            println!("Enter the number of the picture:");
            std::io::stdin()
                .read_line(&mut user_input)
                .expect("Failed to read user input");

            // Parse user input as usize
            let chosen_index: usize = user_input.trim().parse().expect("Invalid input");
            let mut chosen_string = "".to_string();

            // Ensure the chosen index is within bounds
            if chosen_index > 0 && chosen_index <= png_files.len() {
                // Retrieve the chosen string
                chosen_string = png_files[chosen_index - 1].to_string();
                println!("You chose: {}", chosen_string);
            } else {
                println!("Invalid choice");
            }
            for image_path in image_vector {
                if let Err(err) = fs::remove_file(image_path) {
                    eprintln!("Error deleting");
                }
            }
            client_socket
                .send_to(chosen_string.as_bytes(), client_address)
                .await
                .expect("Failed to send");
            let mut user_views = String::new();
            println!("Enter the number of views you want:");
            std::io::stdin()
                .read_line(&mut user_views)
                .expect("Failed to read user input");
            let chosen_views: usize = user_views.trim().parse().expect("Invalid input");
            let chosen_views_bytes: &[u8] = &chosen_views.to_be_bytes();
            client_socket
                .send_to(chosen_views_bytes, client_address)
                .await
                .expect("Failed to send");

            j = 0;
            while j < enc_total {
                if let Ok((_bytes_received, _client_address)) =
                    client_socket.recv_from(&mut request_buffer).await
                {
                    // let packet_string =
                    //     String::from_utf8_lossy(&request_buffer[0.._bytes_received]);
                    let received_message: Message =
                        serde_json::from_slice(&request_buffer[.._bytes_received])
                            .expect("Failed to deserialize message");
                    let recieved = match received_message {
                        Message::Request(request_string) => request_string,
                        Message::DOs(_) => todo!(),
                    };
                    let frag: frag = serde_json::from_str(&recieved).unwrap();
                    enc_total = frag.total_frags_number;
                    request_buffer = [0; 65536];
                    if j == enc_total - 1 {
                        image_chunks.insert(enc_total.try_into().unwrap(), frag.packet);
                    } else {
                        image_chunks.insert(frag.position, frag.packet);
                    }
                    j += 1;
                    client_socket
                        .send_to("Ack".as_bytes(), _client_address)
                        .await
                        .expect("Failed to send");
                }
            }
            let image_chunks_cloned: BTreeMap<_, _> = image_chunks.clone().into_iter().collect();

            for (_key, value) in image_chunks_cloned {
                // println!("Key: {}, Value: {:?}", key, value);
                encrypted_image_data.extend_from_slice(&value);
            }

            remove_trailing_zeros(&mut encrypted_image_data);

            if let Ok(encrypted_image) = image::load_from_memory(&encrypted_image_data) {
                let current_time = Local::now();
                let formatted_time = current_time.format("%Y%m%d%H%M%S").to_string();

                if let Err(err) = encrypted_image.save(format!(
                    "RCV/encrypted{},{}.png",
                    formatted_time, client_sender_address
                )) {
                    eprintln!("Failed to save the image: {}", err);
                } else {
                    println!("Image saved successfully");
                }
            } else {
                println!("Failed to create image from byte stream");
            }
        } else {
            let picture_data = fs::read("japan.png").expect("Failed to read image!");
            let frags = (picture_data.len() / 16384) + 1;
            let mut client_buffer = [0; 65536];

            println!("Sending Image for encryption...");

            for (index, piece) in picture_data.chunks(16384).enumerate() {
                let end = index == frags - 1;
                let frag = frag {
                    views: { 4 },
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
