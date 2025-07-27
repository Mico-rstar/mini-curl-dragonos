/// HTTP响应结构体，包含原始内容、头部、体
pub struct Response {
    pub raw: String,
    pub headers: Vec<String>,
    pub body: String,
}

impl Response {
    /// 解析原始HTTP响应字符串为 Response 结构体
    pub fn parse(raw: String) -> Self {
        if let Some(pos) = raw.find("\r\n\r\n") {
            let (header_str, body_str) = raw.split_at(pos + 4);
            let headers = header_str
                .split("\r\n")
                .filter(|line| !line.is_empty())
                .map(|s| s.to_string())
                .collect();
            let body = body_str.to_string();
            Response { raw, headers, body }
        } else {
            // 没有找到分隔符，全部作为 body
            Response {
                raw: raw.clone(),
                headers: Vec::new(),
                body: raw,
            }
        }
    }
    /// 读取流中所有内容，返回 String
    pub fn read<R: std::io::Read>(stream: &mut R) -> std::io::Result<String> {
        let mut buffer = [0; 1024];
        let mut result = String::new();
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let raw = String::from_utf8_lossy(&buffer[..n]);
                    result += &raw;
                    print!("{raw}");
                },
                Err(e) => {
                    println!("与linux不一致: {:?}", e);
                    break;
                }
            }
        }
        Ok(result)
    }
}
