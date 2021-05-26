use std::thread;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use super::{ConnectionStatus, InitState};

pub fn make_ip(ip1: String, ip2: String, ip3: String, ip4: String) -> String {
    [ip1, ip2, ip3, ip4].join(".")
}

pub fn server_main(ip_address: String, port: String, state: &mut InitState) {
    let ip = [ip_address, port].join(":");
    let listener = TcpListener::bind(&ip).unwrap();
    // Accept connections and process them, spawing a new thread for each one. 
    println!("Server listening on {}", ip);
    for stream in listener.incoming() {
        // We only have to accept one client
        match stream {
            Ok(stream) => {
                state.connection_status = ConnectionStatus::Connected;
                println!("New connection: {}", stream.peer_addr().unwrap());
                thread::spawn(move|| {
                    println!("Connection succeeded")
                });
            },
            Err(e) => {
                // Connection failed
                println!("Error: {}", e);
            }
        }
    }

    drop(listener);
}

pub fn client_main(ip_address: String, port: String) {
    let ip = [ip_address, port].join(":");

    match TcpStream::connect(&ip) {
        Ok(mut stream) => {
            println!("Successfully connected to server at {}", ip);
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
        },
    }
}