use std::{collections::HashMap, fmt::format};

#[derive(Copy, Clone)]
pub enum Method {
    GET,
    POST,
}

// content type
pub enum Contype {
    FORM,     // application/x-www-form-urlencoded
    FORMDATA(String), // multipart/form-data
    JSON,     // application/json
    XML,      // text/xml
    TEXT,     // text/plain
    STREAM,   //application/octet-stream
}

impl Contype {
    pub fn to_string(&self) -> String {
        match self {
            Contype::FORM => String::from("application/x-www-form-urlencoded"),
            Contype::FORMDATA(boundary) => format!("multipart/form-data; boundary={}", boundary),
            Contype::JSON => String::from("application/json"),
            Contype::XML => String::from("text/xml"),
            Contype::TEXT => String::from("text/plain"),
            Contype::STREAM => String::from("application/octet-stream"),
        }
    }
}

struct RequestLine {
    pub method: Method,
    pub path: String,
    pub version: String,
}

pub struct Header {
    pub request_line: Option<RequestLine>,
    header: HashMap<String, String>,
}

impl Header {
    pub fn new() -> Self {
        let mut header = HashMap::new();
        header.insert(
            "User-Agent".to_string(),
            "mini-curl-dragonos/0.1".to_string(),
        );
        header.insert("Accept".to_string(), "*/*".to_string());
        Header {
            request_line: None,
            header,
        }
    }

    pub fn with_request_line(&mut self, method: Method, path: &str, version: &str) -> &mut Self {
        self.request_line = Some(RequestLine {
            method,
            path: path.to_string(),
            version: version.to_string(),
        });
        self
    }

    pub fn content_type(&mut self, value: &str) -> &mut Self {
        self.header
            .insert("Content-Type".to_string(), value.to_string());
        self
    }

    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.header.insert(key.to_string(), value.to_string());
        self
    }

    pub fn to_string(&self) -> String {
        let mut lines = Vec::new();
        if let Some(ref req) = self.request_line {
            let method_str = match req.method {
                Method::GET => "GET",
                Method::POST => "POST",
            };
            lines.push(format!("{} {} {}", method_str, req.path, req.version));
        }
        lines.extend(self.header.iter().map(|(k, v)| format!("{}: {}", k, v)));
        lines.join("\r\n")
    }

    pub fn from(s: &str) -> Self {
        let mut lines = s.lines();
        let mut request_line = None;
        let mut header = HashMap::new();

        if let Some(first) = lines.next() {
            let parts: Vec<&str> = first.split_whitespace().collect();
            if parts.len() == 3 {
                let method = match parts[0] {
                    "GET" => Method::GET,
                    "POST" => Method::POST,
                    _ => Method::GET, // 默认
                };
                request_line = Some(RequestLine {
                    method,
                    path: parts[1].to_string(),
                    version: parts[2].to_string(),
                });
            } else if let Some((key, value)) = first.split_once(": ") {
                header.insert(key.to_string(), value.to_string());
            }
        }

        for line in lines {
            if let Some((key, value)) = line.split_once(": ") {
                header.insert(key.to_string(), value.to_string());
            }
        }

        Header {
            request_line,
            header,
        }
    }
}

#[test]
fn test_header_new_and_setters() {
    let mut header = Header::new();
    header
        .with_request_line(Method::GET, "/index.html", "HTTP/1.1")
        .content_type("application/json")
        .set("Custom-Header", "Value");

    let header_str = header.to_string();
    assert!(header_str.contains("GET /index.html HTTP/1.1"));
    assert!(header_str.contains("User-Agent: mini-curl-dragonos/0.1"));
    assert!(header_str.contains("Accept: */*"));
    assert!(header_str.contains("Content-Type: application/json"));
    assert!(header_str.contains("Custom-Header: Value"));
}

#[test]
fn test_header_from() {
    let raw = "POST /api HTTP/1.1\r\nUser-Agent: mini-curl-dragonos/0.1\r\nAccept: */*\r\nContent-Type: text/plain\r\n";
    let header = Header::from(raw);

    assert!(header.request_line.is_some());
    let req = header.request_line.unwrap();
    match req.method {
        Method::POST => {}
        _ => panic!("Method should be POST"),
    }
    assert_eq!(req.path, "/api");
    assert_eq!(req.version, "HTTP/1.1");
    assert_eq!(
        header.header.get("User-Agent").unwrap(),
        "mini-curl-dragonos/0.1"
    );
    assert_eq!(header.header.get("Accept").unwrap(), "*/*");
    assert_eq!(header.header.get("Content-Type").unwrap(), "text/plain");
}
