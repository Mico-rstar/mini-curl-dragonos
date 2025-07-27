use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseBody {
    Text(String),
    Binary(Vec<u8>),
}


/// HTTP响应结构体，支持文本和二进制类型响应数据
#[derive(Clone, Debug)]
pub struct Response {
    pub raw: Vec<u8>,                     // 原始响应数据
    pub headers: HashMap<String, String>, // 头部字段
    pub body: ResponseBody,            // 响应体
}

impl Response {
    pub fn parse(raw: Vec<u8>) -> Self {
        let delimiter = b"\r\n\r\n";
        if let Some(pos) = raw.windows(delimiter.len()).position(|window| window == delimiter) {
            let header_bytes = &raw[..pos + delimiter.len()];
            let body_bytes = raw[pos + delimiter.len()..].to_vec();

            let header_str = String::from_utf8_lossy(header_bytes);
            let mut headers = HashMap::new();
            for line in header_str.lines().skip(1) {
                if let Some((k, v)) = line.split_once(":") {
                    headers.insert(k.trim().to_string(), v.trim().to_string());
                }
            }

            let body = Self::detect_body(&headers, body_bytes);

            Response {
                raw,
                headers,
                body,
            }
        } else {
            let body = Self::detect_body(&HashMap::new(), raw.clone());
            Response {
                raw: raw.clone(),
                headers: HashMap::new(),
                body,
            }
        }
    }

    fn detect_body(headers: &HashMap<String, String>, body_bytes: Vec<u8>) -> ResponseBody {
        if let Some(content_type) = headers.get("Content-Type") {
            let ct = content_type.to_ascii_lowercase();
            if ct.starts_with("text/")
                || ct.contains("json")
                || ct.contains("xml")
                || ct.contains("html")
            {
                // 文本类型
                if let Ok(s) = String::from_utf8(body_bytes) {
                    ResponseBody::Text(s)
                } else {
                    ResponseBody::Binary(Vec::new())
                }
            } else {
                ResponseBody::Binary(body_bytes)
            }
        } else {
            // 没有 Content-Type，尝试按 utf8 解码
            match String::from_utf8(body_bytes) {
                Ok(s) => ResponseBody::Text(s),
                Err(e) => ResponseBody::Binary(e.into_bytes()),
            }
        }
    }

    /// 读取流中所有内容，返回 Vec<u8>
    pub fn read<R: std::io::Read>(stream: &mut R) -> std::io::Result<Vec<u8>> {
        let mut buffer = [0u8; 1024];
        let mut result = Vec::new();
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    result.extend_from_slice(&buffer[..n]);
                }
                Err(e) => {
                    println!("与linux不一致: {:?}", e);
                    break;
                }
            }
        }
        Ok(result)
    }

    
}
