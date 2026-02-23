#[cfg(test)]
mod tests;
pub mod thread_pool;

use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use crate::thread_pool::ThreadPool;

struct HttpRequest {
    method: String,
    path: String,
    version: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

#[derive(Debug)]
enum ParseError {
    InvalidRequestLine,
    InvalidHeader,
    IncompleteRequest,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequestLine => write!(f, "invalid request line"),
            Self::InvalidHeader => write!(f, "invalid header"),
            Self::IncompleteRequest => write!(f, "incomplete request"),
        }
    }
}

impl std::error::Error for ParseError {}

fn parse_request(request: &str) -> Result<HttpRequest, ParseError> {
    let mut lines = request.lines();

    let request_line = lines.next().ok_or(ParseError::IncompleteRequest)?;

    let (method, rest) = request_line
        .split_once(' ')
        .ok_or(ParseError::InvalidRequestLine)?;
    let (path, version) = rest.split_once(' ').ok_or(ParseError::InvalidRequestLine)?;

    let headers = lines
        .by_ref()
        .take_while(|line| !line.is_empty())
        .map(|line| {
            let (key, value) = line.split_once(':').ok_or(ParseError::InvalidHeader)?;
            Ok((key.trim().to_string(), value.trim().to_string()))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;

    let body = lines.collect::<Vec<_>>().join("\n");
    let body = (!body.is_empty()).then_some(body);

    Ok(HttpRequest {
        method: method.to_string(),
        path: path.to_string(),
        version: version.to_string(),
        headers,
        body,
    })
}

const BAD_REQUEST: &[u8] = b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
const TOO_LARGE: &[u8] = b"HTTP/1.1 413 Content Too Large\r\nContent-Length: 0\r\n\r\n";
const MAX_HEADERS_SIZE: usize = 8 * 1024;       // 8 KB
const MAX_BODY_SIZE: usize = 1 * 1024 * 1024;  // 1 MB

fn handle_connection(mut stream: TcpStream) {
    if try_handle(&stream).is_err() {
        let _ = stream.write_all(BAD_REQUEST);
    }
}

fn try_handle(mut stream: &TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(stream);

    let mut raw_headers = String::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
        if raw_headers.len() + line.len() > MAX_HEADERS_SIZE {
            stream.write_all(TOO_LARGE)?;
            return Ok(());
        }
        raw_headers.push_str(&line);
    }

    let content_length: usize = raw_headers
        .lines()
        .skip(1)
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);

    if content_length > MAX_BODY_SIZE {
        stream.write_all(TOO_LARGE)?;
        return Ok(());
    }

    let mut body_bytes = vec![0u8; content_length];
    reader.read_exact(&mut body_bytes)?;
    let body_str = String::from_utf8(body_bytes)?;

    let full_request = format!("{}\r\n\r\n{}", raw_headers.trim_end(), body_str);

    let request = parse_request(&full_request)?;

    let body = format!("Method: {}, Path: {}", request.method, request.path);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;

    Ok(())
}

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port = env::var("PORT").expect("Missing PORT");
    let url = format!("127.0.0.1:{}", port);

    let pool = ThreadPool::new(4).expect("Failed to create pool");

    let listener = TcpListener::bind(&url)?;
    println!("Listening on {}", url);

    for stream in listener.incoming() {
        let stream = stream?;
        pool.execute(move || handle_connection(stream));
    }

    Ok(())
}
