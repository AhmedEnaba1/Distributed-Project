use async_std::net::UdpSocket;
use chrono::prelude::*;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
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


async fn client_listener(client_listener_socket: UdpSocket) {
    let mut buffer = [0; BUFFER_SIZE];
    let mut ack_buffer = [0; BUFFER_SIZE];
    let mut my_clients: Vec<String> = Vec::new();
    while let Ok((_bytes_received, client_address)) =
        client_listener_socket.recv_from(&mut buffer).await
    {
        loop {
            if unsafe { STATE == true } {
                break;
            }
        }
        let message_user = String::from_utf8_lossy(&buffer[.._bytes_received]);
        if (message_user.starts_with("VIEWS")) {
            let mut p = true;
            while (p) {
                println!("Do you want to give this client more views?");
                println!("1.Yes");
                println!("2.No");
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");
                if (input == "1") {
                    p = false;
                    println!("How many views to give them?");
                    let mut input = String::new();
                    // Read a line from standard input
                    io::stdin()
                        .read_line(&mut input)
                        .expect("Failed to read line");

                    // Trim whitespace and convert the string to a slice of bytes
                    let index: &[u8] = input.trim().as_bytes();
                    client_listener_socket.send_to(index, client_address);
                } else if (input == "2") {
                    p = false;
                    let index: &[u8] = "0".trim().as_bytes();
                    client_listener_socket.send_to(index, client_address);
                } else {
                }
            }
            continue;
        }
        if message_user.starts_with("UPDATE") {
            println!("Users:");
            for (index, user) in my_clients.iter().enumerate() {
                println!("{}: {}", index + 1, user);
            }

            // Ask the user for a number
            let mut user_input = String::new();
            let mut user_to_send = "".to_string();

            loop {
                println!("Enter the number corresponding to the user:");
                user_input.clear();

                io::stdin()
                    .read_line(&mut user_input)
                    .expect("Failed to read line");

                match user_input.trim().parse::<usize>() {
                    Ok(index) if index > 0 && index <= my_clients.len() => {
                        // User entered a valid number
                        let selected_user = &my_clients[index - 1];
                        println!("You selected: {}", selected_user);
                        user_to_send = selected_user.to_string();
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
                        let x: &[u8] = &views.to_be_bytes();
                        let parts: Vec<&str> = user_to_send.split(':').collect();
                        // Take the first part (IP address)
                        let mut user_s = "".to_string();
                        if let Some(ip) = parts.get(0) {
                            // Create a new string with the IP and the desired port
                            user_to_send = format!("{}:8085", ip);
                            user_s = user_to_send;
                        }
                        client_listener_socket
                            .send_to("Sig".as_bytes(), user_s)
                            .await;
                        let (_, car) = client_listener_socket
                            .recv_from(&mut ack_buffer)
                            .await
                            .expect("Couldn't recieve index");
                        client_listener_socket.send_to(x, car).await;
                        break;
                    }
                    Err(_) => {
                        // Invalid input, ask the user again
                        println!("Invalid input. Please enter a valid number.");
                    }
                }
            }
            client_listener_socket
                .send_to("Sig".as_bytes(), client_address)
                .await;
            continue;
        }
        if message_user.starts_with("IGET") {
            client_listener_socket
                .send_to("buf".as_bytes(), client_address)
                .await;
            client_listener_socket.recv_from(&mut buffer).await;
            if let Ok(entries) = fs::read_dir("other_images/") {
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
                        let encoded_image = file_as_image_buffer(format!(
                            "other_images/{}",
                            file_name.trim_matches('"')
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
                        decoded_image_data[0] = decoded_image_data[0] + buffer[7];
                        println!("New Views:{}", decoded_image_data[0]);
                        if let Ok(encrypted_image) =
                            image::load_from_memory(&decoded_image_data[1..])
                        {
                            let (width, height) = encrypted_image.dimensions();
                            println!("Image dimensions: {} x {}", width, height);

                            if let Err(err) = encrypted_image.save("other_images/decoded.png") {
                                eprintln!("Failed to save the image: {}", err);
                            } else {
                                println!("Image saved successfully");
                            }
                        } else {
                            println!("Failed to create image from byte stream");
                        }

                        if let Ok(_image) = image::open("other_images/decoded.png") {
                            println!("Viewing image: decoded.png");
                            if let Ok(_) = open::that("other_images/decoded.png") {
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
                        save_image_buffer(
                            result,
                            format!("other_images/{}", file_name.trim_matches('"')),
                        );
                    }
                } else {
                    println!("No matching PNG files found.");
                }

                continue;
            }
        }

        let png_files: Vec<String> = fs::read_dir("my_images/")
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
        println!("Len:{:?}", png_file_legnth);
        client_listener_socket
            .send_to(png_file_legnth, client_address)
            .await
            .expect("Failed to send data to server");
        let mut iterate = 0;
        while iterate < png_files.len() {
            println!("{}", &png_files[iterate]);
            let img = image::open(format!(
                "my_images/{}",
                &png_files[iterate].trim_matches('"')
            ))
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

            let packet_number = (image_data.len() / MAX_CHUNCK) + 1;
            println!("Packets:{}", packet_number);
            for (index, piece) in image_data.chunks(MAX_CHUNCK).enumerate() {
                let is_last_piece = index == packet_number - 1;
                let chunk = Chunk {
                    views: unsafe { VIEWS },
                    total_packet_number: packet_number,
                    position: if is_last_piece {
                        -1
                    } else {
                        index.try_into().unwrap()
                    },
                    packet: {
                        let mut packet_array = [0; MAX_CHUNCK];
                        packet_array[..piece.len()].copy_from_slice(piece);
                        packet_array
                    },
                };
                let serialized = serde_json::to_string(&chunk).unwrap();
                client_listener_socket
                    .send_to(&serialized.as_bytes(), client_address)
                    .await
                    .expect("Failed to send piece to middleware");
                let (num_bytes_received, _) = client_listener_socket
                    .recv_from(&mut ack_buffer)
                    .await
                    .expect("Failed to receive acknowledgement from server");

                //let received_string = String::from_utf8_lossy(&client_buffer[..num_bytes_received]);
                //println!("Received {}", received_string);
            }
            iterate += 1;
        }
        let png_files_json =
            serde_json::to_vec(&png_files).expect("Failed to serialize png_files to JSON");
        client_listener_socket
            .send_to(&png_files_json, client_address)
            .await
            .expect("Failed to send piece to client");
        let (num_bytes_received, cadd) = client_listener_socket
            .recv_from(&mut buffer)
            .await
            .expect("Failed to receive acknowledgement from server");
        my_clients.push(cadd.to_string());
        let filename = format!(
            "my_images/{}",
            String::from_utf8_lossy(&buffer[..num_bytes_received]).to_string()
        );
        let (num_bytes_received, _) = client_listener_socket
            .recv_from(&mut buffer)
            .await
            .expect("Failed to receive acknowledgement from server");
        let requested_views = u8::from_le_bytes([buffer[7]]);
        println!("RVs:{}", requested_views);

        //Start Encryption
        let image_data = fs::read(filename).expect("Failed to read the image file");
        let middleware_address = "127.0.0.8:12345"; // Replace with the actual middleware address and port
                                                    //sleep(Duration::from_millis(5000)).await;

        let packet_number = (image_data.len() / MAX_CHUNCK) + 1;
        for (index, piece) in image_data.chunks(MAX_CHUNCK).enumerate() {
            let is_last_piece = index == packet_number - 1;
            let chunk = Chunk {
                views: requested_views,
                total_packet_number: packet_number,
                position: if is_last_piece {
                    -1
                } else {
                    index.try_into().unwrap()
                },
                packet: {
                    let mut packet_array = [0; MAX_CHUNCK];
                    packet_array[..piece.len()].copy_from_slice(piece);
                    packet_array
                },
            };
            println!("Index:{}", index);
            let serialized = serde_json::to_string(&chunk).unwrap();

            client_listener_socket
                .send_to(&serialized.as_bytes(), middleware_address)
                .await
                .expect("Failed to send piece to middleware");

            let (num_bytes_received, _) = client_listener_socket
                .recv_from(&mut buffer)
                .await
                .expect("Failed to receive acknowledgement from server");
            let received_string = String::from_utf8_lossy(&buffer[..num_bytes_received]);
            println!("Received {}", received_string);
        }
        println!("Finished All Packets");
        println!("{}", packet_number);

        let mut encrypted_image_data: Vec<u8> = Vec::new();
        let mut image_chunks = HashMap::<i16, PacketArray>::new();
        let mut j = 0;
        let mut enecrypted_image_packet_number = packet_number;

        while j < enecrypted_image_packet_number {
            println!("j : {}  EIPN: {}", j, enecrypted_image_packet_number);
            if let Ok((_bytes_received, _client_address)) =
                client_listener_socket.recv_from(&mut buffer).await
            {
                let packet_string = String::from_utf8_lossy(&buffer[0.._bytes_received]);
                let deserialized: Chunk = serde_json::from_str(&packet_string).unwrap();
                enecrypted_image_packet_number = deserialized.total_packet_number;
                shift_left(&mut buffer, _bytes_received);

                if j == enecrypted_image_packet_number - 1 {
                    image_chunks.insert(
                        enecrypted_image_packet_number.try_into().unwrap(),
                        deserialized.packet,
                    );
                    println!("Ana 5aragt {}", j);
                } else {
                    image_chunks.insert(deserialized.position, deserialized.packet);
                }
                j += 1;
                client_listener_socket
                    .send_to("Ack".as_bytes(), _client_address)
                    .await
                    .expect("Failed to send");
            }
        }
        println!("Ana 5aragt");
        let image_chunks_cloned: BTreeMap<_, _> = image_chunks.clone().into_iter().collect();

        for (_key, value) in image_chunks_cloned {
            // println!("Key: {}, Value: {:?}", key, value);
            encrypted_image_data.extend_from_slice(&value);
        }

        remove_trailing_zeros(&mut encrypted_image_data);

        //End of Encryption

        let packet_number = (encrypted_image_data.len() / MAX_CHUNCK) + 1;
        for (index, piece) in encrypted_image_data.chunks(MAX_CHUNCK).enumerate() {
            let is_last_piece = index == packet_number - 1;
            let chunk = Chunk {
                views: requested_views,
                total_packet_number: packet_number,
                position: if is_last_piece {
                    -1
                } else {
                    index.try_into().unwrap()
                },
                packet: {
                    let mut packet_array = [0; MAX_CHUNCK];
                    packet_array[..piece.len()].copy_from_slice(piece);
                    packet_array
                },
            };
            println!("Index:{}", index);
            let serialized = serde_json::to_string(&chunk).unwrap();

            client_listener_socket
                .send_to(&serialized.as_bytes(), client_address)
                .await
                .expect("Failed to send piece to middleware");

            let (num_bytes_received, _) = client_listener_socket
                .recv_from(&mut buffer)
                .await
                .expect("Failed to receive acknowledgement from server");
            let received_string = String::from_utf8_lossy(&buffer[..num_bytes_received]);
            println!("Received {}", received_string);
        }
    }
}
