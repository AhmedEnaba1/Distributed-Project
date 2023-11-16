// use std::net::{UdpSocket, SocketAddr};
// use std::thread;
// use std::sync::mpsc;
// use std::time::Duration;

// fn main() {
//     // Create a UDP socket for communication with the server
//     let socket = UdpSocket::bind("127.223.3.79:8080").expect("Failed to bind to client address");

//     // Create channels for communication between the main client thread and the middleware thread
//     let (request_sender, request_receiver): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();
//     let (reply_sender, reply_receiver): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();

//     // Spawn the middleware thread
//     thread::spawn(move || {
//         // Define the server's IP address and port
//         let server_ip: SocketAddr = "127.107.2.56:8080".parse().expect("Invalid server address");
//         // let server2_ip: SocketAddr = "127.105.3.1:8080".parse().expect("Invalid server2 address");
//         // let server3_ip: SocketAddr = "127.106.3.1:8080".parse().expect("Invalid server3 address");


//         loop {
//             // Receive a request from the main client thread
//             if let Ok(request) = request_receiver.recv() {
//                 // Send the request to the server
//                 socket.send_to(request.as_bytes(), server_ip).expect("Failed to send request to server");

//                 // Receive the server's reply
//                 let mut buf = [0; 1024];
//                 let (amt, _) = socket.recv_from(&mut buf).expect("Failed to receive data from server");
//                 let reply = String::from_utf8_lossy(&buf[..amt]).to_string();

//                 // Send the reply back to the main client thread
//                 reply_sender.send(reply).expect("Failed to send reply to main thread");
//             }
//         }
//     });

//     loop {
//         request_sender.send("Hello, Server!".to_string()).expect("Failed to send request from the main thread");
//         println!("Sending Request");
//         thread::sleep(Duration::from_secs(3)); // Sleep for 10 seconds
//         // Check for replies and print if received
//         if let Ok(reply) = reply_receiver.try_recv() {
//             println!("Received reply from middleware: '{}'", reply);
//         }
//         else{
//             continue;
//         }
//     }
// }

// use async_std::net::UdpSocket;
// use std::net::SocketAddr;
// use std::time::Duration;
// use tokio::time::sleep;
// use tokio::signal::unix::{signal, SignalKind};
// use tokio::sync::broadcast;
// use std::thread;
// use std::sync::{Arc, Mutex};
// use tokio::time::timeout;

// async fn middleware_task(mut middleware_socket: UdpSocket) {
//     let server_addresses = ["127.0.0.2:21112"/* , "127.0.0.3:21111", "127.0.0.4:21113"*/];
//     let mut buffer = [0; 1024];
//     let mut ack_buffer = [0; 1024];
//     //let middleware_address: SocketAddr = "127.0.0.8:12345".parse().expect("Failed to parse middleware address");

//     loop {
//         if let Ok((_bytes_received, client_address)) =
//             middleware_socket.recv_from(&mut buffer).await
//         {
//             println!("Yo1");
//             println!("Yo2");
//             let server_socket = UdpSocket::bind("127.0.0.8:4000")
//                 .await
//                 .expect("Failed to bind server socket");
//             for server_address in &server_addresses {
//                 let server_address: SocketAddr = server_address
//                     .parse()
//                     .expect("Failed to parse server address");
//                 //server_socket
//                 //    .connect(&server_address)
//                 //    .await
//                 //    .expect("Failed to connect to the server");
//                 println!("Yo3");
//                 server_socket
//                     .send_to(&buffer, &server_address)
//                     .await
//                     .expect("Failed to send data to server");
//                 println!("Yo4");
//             }
//             let timeout_duration = Duration::from_secs(1);

//             match timeout(timeout_duration, server_socket.recv_from(&mut ack_buffer)).await {
//                 Ok(Ok((ack_bytes_received, server_address))) => {
//                 }
//                 Ok(Err(e)) => {
//                 }
//                 Err(_) => {
//                 }
//             }
//             middleware_socket
//                 .send_to(&ack_buffer, client_address)
//                 .await
//                 .expect("Failed to send acknowledgment to client");
//             //println!("Yo6");

//             // Sleep to give time for the server to send the acknowledgment
//             sleep(Duration::from_millis(10)).await;

//             // Clear the buffer for the next request
//             buffer = [0; 1024];
//             ack_buffer = [0; 1024];
//         }
//         }
//     }

// // async fn register_user(client_socket: UdpSocket, dos_address: &str, username: &str,usertype: &str) {
// //     let registration_message = format!("REGISTER:{}:{}", username,usertype);
// //     client_socket.send_to(registration_message.as_bytes(), dos_address).await.expect("Failed to send registration request");

// //     let mut response_buffer = [0; 1024];
// //     let (bytes_received, _dos_address) = client_socket.recv_from(&mut response_buffer).await.expect("Failed to receive response");
// //     let response = String::from_utf8_lossy(&response_buffer[..bytes_received]);
// //     println!("Registration response: {}", response);
// // }

// // async fn query_online_users(client_socket: UdpSocket, middleware_address: &str) {
// //     // Send a query message to request the list of online users
// //     client_socket.send_to("QUERY".as_bytes(), middleware_address).await.expect("Failed to send query request");

// //     let mut response_buffer = [0; 1024];
// //     let (bytes_received, _middleware_address) = client_socket.recv_from(&mut response_buffer).await.expect("Failed to receive response");
// //     let response = String::from_utf8_lossy(&response_buffer[..bytes_received]);
// //     println!("Online users: {}", response);
// // }
   

// #[tokio::main]
// async fn main() {
//     let dos_address= "127.0.0.255:12345";
//     let middleware_address: SocketAddr = "127.0.0.8:12345".parse().expect("Failed to parse middleware address");
//     let client_socket = UdpSocket::bind("127.0.0.8:0").await.expect("Failed to bind client socket");
//     //let client_socket_register = UdpSocket::bind("127.0.0.8:8090").await.expect("Failed to bind client socket");
//     //let client_socket_query = UdpSocket::bind("127.0.0.8:8091").await.expect("Failed to bind client socket");
//     //register_user(client_socket_register,dos_address, "Client1","Client").await;
//     //println!("Finished Registry");
//     //query_online_users(client_socket_query,dos_address).await;
//     let middleware_socket = UdpSocket::bind(&middleware_address).await.expect("Failed to bind middleware socket");

//     tokio::spawn(middleware_task(middleware_socket));
//     let dos_address_clone = dos_address.clone(); // Assuming `dos_address` is defined elsewhere
//     let termination = Arc::new(Mutex::new(0));
//     let termination_clone = Arc::clone(&termination);

//     let (tx, _) = tokio::sync::broadcast::channel::<()>(1);
//     let runtime = tokio::runtime::Builder::new_multi_thread()
//         .enable_all()
//         .build()
//         .expect("Failed to build Tokio runtime");

//     let mut signal = signal(SignalKind::interrupt()).expect("Failed to create signal handler");
//     tokio::spawn(async move {
//         signal.recv().await;
//         println!("Received termination signal");

//         //let unregister_message = "UNREGISTER";
//         //let dos_socket = UdpSocket::bind("127.0.0.8:9000").await.expect("Failed to bind socket");
//         //dos_socket
//         //    .send_to(unregister_message.as_bytes(), dos_address_clone)
//         //    .await
//         //    .expect("Failed to send unregister message");
// //
//         // Notify other tasks waiting for the signal
//         //let _ = tx.send(());
//         *termination_clone.lock().unwrap() = 1;
//         tokio::time::sleep(Duration::from_secs(2)).await;

//         // Exit the application
//         std::process::exit(0);
//         //return;
//     });

//     while *termination.lock().unwrap() == 0 {
//         // Code to trigger a request, perhaps on a button press or any other event
//         // For demonstration purposes, a message is sent when 'Enter' key is pressed
//         println!("Press Enter to send a Request");
//         let mut input = String::new();
//         std::io::stdin().read_line(&mut input).expect("Failed to read line");
//         if input.trim() == "" {
//             for i in 1..2
//             {
//             if(i%90==0)
//             {
//                 sleep(Duration::from_millis(10)).await;
//             }
//             let client_message = "Request from Client 1!";
//             let middleware_address = "127.0.0.8:12345"; // Replace with the actual middleware address and port
//            //sleep(Duration::from_millis(5000)).await;
//             client_socket
//             .send_to(client_message.as_bytes(), middleware_address)
//             .await
//             .expect("Failed to send request to middleware");
   
//             // Receive response from the server (the first one)
//             let mut client_buffer = [0; 1024];
//             client_socket
//             .recv_from(&mut client_buffer)
//             .await
//             .expect("Failed to receive response from server");
//             let response = String::from_utf8_lossy(&client_buffer);
//             println!("Client received response from server: {}", response);
//             }
//         }
//         if input.trim() == "Q"
//         {
//            //let unregister_message = "UNREGISTER";
//            //let dos_socket = UdpSocket::bind("127.0.0.8:9001").await.expect("Failed to bind socket");
//            //dos_socket
//            //.send_to(unregister_message.as_bytes(), dos_address_clone)
//            //.await
//            //.expect("Failed to send unregister message");
//             return;

//         }
//     }

//     //middleware_task_handle.await.expect("Middleware task failed");
// }

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
    let server_addresses = ["127.0.0.2:21112"];
    let mut buffer = [0; 1024];
    let mut ack_buffer = [0; 1024];

    loop {
        if let Ok((_bytes_received, client_address)) = middleware_socket.recv_from(&mut buffer).await {
            let server_socket = UdpSocket::bind("127.0.0.8:4000")
                .await
                .expect("Failed to bind server socket");

            for server_address in &server_addresses {
                let server_address: SocketAddr = server_address.parse().expect("Failed to parse server address");

                let request_message = Message::Request("Request from Client 1!".to_string());
                let serialized_request = serde_json::to_string(&request_message).expect("Failed to serialize request");

                server_socket
                    .send_to(serialized_request.as_bytes(), &server_address)
                    .await
                    .expect("Failed to send data to server");
            }

            let timeout_duration = Duration::from_secs(1);

            match timeout(timeout_duration, server_socket.recv_from(&mut ack_buffer)).await {
                Ok(Ok((_ack_bytes_received, _server_address))) => {
                    // Handle acknowledgment if needed
                }
                Ok(Err(_e)) => {
                    // Handle acknowledgment error if needed
                }
                Err(_) => {
                    // Handle timeout
                }
            }

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
    let middleware_address: SocketAddr = "127.0.0.8:12345".parse().expect("Failed to parse middleware address");
    let client_socket = UdpSocket::bind("127.0.0.8:0").await.expect("Failed to bind client socket");
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

                let request_message = Message::Request("Request from Client 1!".to_string());
                let serialized_request = serde_json::to_string(&request_message).expect("Failed to serialize request");

                client_socket
                    .send_to(serialized_request.as_bytes(), middleware_address)
                    .await
                    .expect("Failed to send request to middleware");

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

        

