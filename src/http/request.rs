use std::collections::HashMap;

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug)]
pub enum ParseError {
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

pub fn parse_request(request: &str) -> Result<HttpRequest, ParseError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_get_request() {
        let raw = "GET /hello HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain\r\n\r\n";
        let req = parse_request(raw).expect("should parse");
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/hello");
        assert_eq!(req.version, "HTTP/1.1");
        assert_eq!(
            req.headers.get("Host").map(|s| s.as_str()),
            Some("localhost")
        );
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
}
