pub mod thread_pool;

use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use crate::thread_pool::ThreadPool;

struct HttpRequest {
    method: String,
    path: String,
    version: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

enum ParseError {
    InvalidRequestLine,
    InvalidHeader,
    IncompleteRequest,
    UnsupportedMethod,
}

fn parse_request(request: &str) -> Result<HttpRequest, ParseError> {
    let mut lines = request.lines();

    let request_line = lines.next().ok_or(ParseError::IncompleteRequest)?;
    let (method, path, version) = {
        let mut parts = request_line.split_whitespace();
        let method = parts
            .next()
            .ok_or(ParseError::InvalidRequestLine)?
            .to_string();
        let path = parts
            .next()
            .ok_or(ParseError::InvalidRequestLine)?
            .to_string();
        let version = parts
            .next()
            .ok_or(ParseError::InvalidRequestLine)?
            .to_string();
        (method, path, version)
    };

    let headers = lines
        .by_ref()
        .take_while(|line| !line.is_empty())
        .map(|line| {
            let mut header_parts = line.splitn(2, ':');
            let key = header_parts
                .next()
                .ok_or(ParseError::InvalidHeader)?
                .trim()
                .to_string();
            let value = header_parts
                .next()
                .ok_or(ParseError::InvalidHeader)?
                .trim()
                .to_string();
            Ok((key, value))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;

    let body = {
        let body = lines.collect::<Vec<&str>>().join("\n");
        (!body.is_empty()).then_some(body)
    };

    Ok(HttpRequest {
        method,
        path,
        version,
        headers,
        body,
    })
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = Vec::new();
    let mut temp_buffer = [0; 1024];

    let result = loop {
        match stream.read(&mut temp_buffer) {
            Ok(0) => break Err(ParseError::IncompleteRequest),
            Ok(n) => buffer.extend_from_slice(&temp_buffer[..n]),
            Err(_) => break Err(ParseError::IncompleteRequest),
        }

        if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
            break Ok(());
        }
    };

    if let Err(_) = result {
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    let request_str = String::from_utf8_lossy(&buffer);
    match parse_request(&request_str) {
        Ok(request) => {
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\nMethod: {}, Path: {}",
                request.method.len() + request.path.len() + 10,
                request.method,
                request.path
            );
            let _ = stream.write_all(response.as_bytes());
        }
        Err(_) => {
            let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(response.as_bytes());
        }
    }
}

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port = env::var("PORT").expect("Missing PORT");
    let url = format!("127.0.0.1:{}", port);

    let pool = ThreadPool::new(4).expect("Failed to create pool");

    if let Ok(listener) = TcpListener::bind(&url) {
        println!("Successfully bind to {}", url);
        println!("Waiting for connections...");

        for request in listener.incoming() {
            let req = request.unwrap();

            pool.execute(move || handle_connection(req))
        }
    } else {
        println!("Failed to bind to port {}", port);
    }

    Ok(())
}
