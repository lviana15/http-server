use std::collections::HashMap;

pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl HttpResponse {
    pub fn new(status_code: u16, status_text: &str) -> Self {
        Self {
            status_code,
            status_text: status_text.to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn ok(body: &str) -> Self {
        let mut res = Self::new(200, "OK");
        res.body = Some(body.to_string());
        res.headers
            .insert("Content-Type".to_string(), "text/plain".to_string());
        res
    }

    pub fn not_found() -> Self {
        Self::new(404, "Not Found")
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut response = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text);

        let mut headers = self.headers;
        let body_bytes = if let Some(body) = self.body {
            body.into_bytes()
        } else {
            Vec::new()
        };

        headers.insert("Content-Length".to_string(), body_bytes.len().to_string());

        for (k, v) in headers {
            response.push_str(&format!("{}: {}\r\n", k, v));
        }

        response.push_str("\r\n");
        let mut response_bytes = response.into_bytes();
        response_bytes.extend(body_bytes);

        response_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_response() {
        let res = HttpResponse::new(500, "Internal Server Error");
        assert_eq!(res.status_code, 500);
        assert_eq!(res.status_text, "Internal Server Error");
        assert!(res.headers.is_empty());
        assert!(res.body.is_none());
    }

    #[test]
    fn test_ok_response() {
        let res = HttpResponse::ok("Hello");
        assert_eq!(res.status_code, 200);
        assert_eq!(res.status_text, "OK");
        assert_eq!(res.headers.get("Content-Type").unwrap(), "text/plain");
        assert_eq!(res.body.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_not_found_response() {
        let res = HttpResponse::not_found();
        assert_eq!(res.status_code, 404);
        assert_eq!(res.status_text, "Not Found");
        assert!(res.headers.is_empty());
        assert!(res.body.is_none());
    }

    #[test]
    fn test_into_bytes_formats_correctly() {
        let mut res = HttpResponse::new(200, "OK");
        res.body = Some("body content".to_string());
        res.headers
            .insert("X-Custom".to_string(), "Value".to_string());

        let bytes = res.into_bytes();
        let s = String::from_utf8(bytes).unwrap();

        assert!(s.starts_with("HTTP/1.1 200 OK\r\n"));
        // headers order is not guaranteed because of HashMap, so check presence
        assert!(s.contains("Content-Length: 12\r\n"));
        assert!(s.contains("X-Custom: Value\r\n"));
        assert!(s.ends_with("\r\n\r\nbody content"));
    }

    #[test]
    fn test_into_bytes_without_body() {
        let res = HttpResponse::new(404, "Not Found");

        let bytes = res.into_bytes();
        let s = String::from_utf8(bytes).unwrap();

        assert!(s.starts_with("HTTP/1.1 404 Not Found\r\n"));
        assert!(s.contains("Content-Length: 0\r\n"));
        assert!(s.ends_with("\r\n\r\n"));
    }
}
