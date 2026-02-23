use super::parse_request;

#[test]
fn test_basic_get_request() {
    let raw = "GET /hello HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain\r\n\r\n";
    let req = parse_request(raw).expect("should parse");
    assert_eq!(req.method, "GET");
    assert_eq!(req.path, "/hello");
    assert_eq!(req.version, "HTTP/1.1");
    assert_eq!(req.headers.get("Host").map(|s| s.as_str()), Some("localhost"));
    assert!(req.body.is_none());
}

#[test]
fn test_post_with_body() {
    let raw = "POST /submit HTTP/1.1\r\nContent-Length: 13\r\n\r\nhello, world!";
    let req = parse_request(raw).expect("should parse");
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/submit");
    assert_eq!(req.body.as_deref(), Some("hello, world!"));
}

#[test]
fn test_empty_request_fails() {
    let result = parse_request("");
    assert!(result.is_err());
}
