use crate::response::{self, Response};
use crate::structs::{Contype, Header, Method};
use crate::{file_io, parser};
use rustls_pki_types::ServerName;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::u8;

/*
TO_DO
    - 流式输出到控制台
    - 判断结束条件: 将请求头改为connection: close后解决
*/

pub struct Request {
    data: Option<String>,
    header: Header,
    url: url::Url,
    response: Option<Response>,
    formdata: Option<Vec<u8>>,
    ctype: Contype,
}

impl Request {
    pub fn new(_url: &url::Url) -> Self {
        Request {
            data: None,
            header: Header::new(),
            url: _url.clone(),
            response: None,
            formdata: None,
            ctype: Contype::FORM,
        }
    }

    fn construct_header(&mut self, method: Method) -> &mut Header {
        // 构造请求路径 (如果路径为空则使用 "/")
        let path = if self.url.path().is_empty() {
            "/"
        } else {
            self.url.path()
        };

        let host = self
            .url
            .host_str()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "URL must have a host",
            ))
            .unwrap();

        // 构造查询字符串 (如果有的话)
        let query = self
            .url
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default();

        self.header
            .with_request_line(method, path, "HTTP/1.1")
            .set("Host", host)
            .set("Content-Type", &self.ctype.to_string())

    }

    fn match_method(method_str: String) -> Result<Method, Box<dyn std::error::Error>> {
        match method_str.as_str() {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            _ => Err("unsupported method".into()),
        }
    }

    pub fn set_header(&mut self, _h: &String) -> &mut Self {
        self.header = Header::from(_h);
        self
    }

    pub fn add_item_to_header(&mut self, item: &str) -> &mut Self {
        let parts: Vec<&str> = item.splitn(2, ':').collect();
        let (key, value) = if parts.len() == 2 {
            (parts[0].trim(), parts[1].trim())
        } else {
            (item.trim(), "")
        };
        self.header.set(key, value);
        self
    }

    pub fn set_data(&mut self, _d: &String) -> &mut Self {
        self.data = Some(_d.clone());
        self.ctype = Contype::JSON;
        self
    }

    pub fn set_formdata(&mut self, d: &[u8], boundary: String) -> &mut Self {
        self.formdata = Some(d.to_vec());
        self.ctype = Contype::FORMDATA(boundary);
        self
    }

    // http get/post
    pub fn http_do(&mut self, method_str: String) -> Result<(), Box<dyn std::error::Error>> {
        let addrs = parser::to_adders(&self.url)?;
        let method = Self::match_method(method_str)?;
        match method {
            Method::GET => self.get(&addrs),
            Method::POST => self.post(&addrs),
        }
    }

    pub fn get(&mut self, addrs: &Vec<SocketAddr>) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(&addrs[..])?;

        // 构造完整的请求头
        self.construct_header(Method::GET)
            .set("Connection", "close");

        let request = self.header.to_string() + "\r\n\r\n";

        // 发送请求
        stream.write_all(request.as_bytes())?;

        self.fetch_response(&mut stream)?;

        Ok(())
    }

    pub fn post(&mut self, addrs: &Vec<SocketAddr>) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(&addrs[..])?;

        // 构造完整的请求头和请求体
        self.construct_header(Method::POST)
            .set("Connection", "close");
        
        let mut body: Option<&[u8]> = None;
        match self.ctype {
            Contype::FORMDATA(_) => {
                self.header.set(
                    "Content-Length",
                    &self.formdata.clone().unwrap().len().to_string(),
                );

                if let Some(ref formdata) = self.formdata {
                    body = Some(formdata.as_slice());
                }
            }
            Contype::JSON => {
                self.header.set(
                    "Content-Length",
                    &self.data.clone().unwrap().len().to_string(),
                );

                if let Some(ref data) = self.data {
                    body = Some(data.as_bytes());
                }
            }

            _ => (),
        }
        let request = self.header.to_string() + "\r\n\r\n";


        // 发送请求头
        stream.write_all(request.as_bytes())?;
        // 发送请求体
        if let Some(body) = body {
            stream.write_all(body)?;
        }

        self.fetch_response(&mut stream)?;

        Ok(())
    }

    // https get/post
    pub fn https_do(&mut self, method_str: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut last_error = None;
        let method = Self::match_method(method_str)?;
        let addrs = parser::to_adders(&self.url)?;
        for addr in addrs {
            let ip = addr.ip().to_string();
            let port = addr.port();

            match self.try_https(ip, port, method) {
                Ok(()) => return Ok(()),        // 成功则立即返回
                Err(e) => last_error = Some(e), // 失败则记录错误
            }
        }

        // 所有地址都失败，返回最后一个错误
        last_error.map_or_else(
            || Ok(()),  // 如果 last_error 为空，返回 Ok(())
            |e| Err(e), // 否则返回最后一个错误
        )
    }

    fn try_https(
        &mut self,
        host: String,
        port: u16,
        method: Method,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 构造完整的请求头
        self.construct_header(method.clone());

        // 配置 `rustls` 客户端以跳过验证
        // 创建一个危险的客户端配置构建器，允许不安全的证书验证
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerification)) // 使用自定义的空验证器
            .with_no_client_auth();

        // let mut root_cert_store = rustls::RootCertStore::empty();

        // // 加载操作系统原生的根证书
        // for cert in rustls_native_certs::load_native_certs()? {
        //     root_cert_store.add(cert)?;
        // }

        // // 创建 TLS 客户端配置
        // let config = rustls::ClientConfig::builder()
        //     .with_root_certificates(root_cert_store) // 设置信任的根证书
        //     .with_no_client_auth(); // 指定客户端不需要提供证书进行验证

        let server_name: ServerName = host.to_string().try_into().map_err(|_| "无效的DNS名称")?;
        let mut client_conn = rustls::ClientConnection::new(Arc::new(config), server_name)?;

        let mut tcp_stream = TcpStream::connect((host, port))?;
        // tcp_stream.set_read_timeout(Some(Duration::new(3, 0)));

        let mut tls_stream = rustls::Stream::new(&mut client_conn, &mut tcp_stream);

        let request;
        match method {
            Method::GET => request = self.header.to_string(),
            Method::POST => {
                request = self.header.to_string() + &self.data.clone().unwrap_or_default()
            }
        }

        tls_stream.write_all(request.as_bytes())?;
        tls_stream.flush()?;

        self.fetch_response(&mut tls_stream)?;

        Ok(())
    }

    fn fetch_response<R: std::io::Read>(
        &mut self,
        stream: &mut R,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = Response::read(stream)?;
        self.response = Some(Response::parse(raw));
        if let Some(resq) = &self.response {
            match &resq.body {
                response::ResponseBody::Text(text) => {
                    println!("{}", text);
                }
                response::ResponseBody::Binary(_) => {
                    println!("Warning: Binary output can mess up your terminal.");
                    println!("Warning: or consider '--output <FILE>' to save to a file.");
                }
            }
        }
        Ok(())
    }

    pub fn response_output(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(resp) = &self.response {
            match &resp.body {
                response::ResponseBody::Text(text) => {
                    file_io::write_string_to_file(path, text)?;
                }
                response::ResponseBody::Binary(data) => {
                    file_io::write_bytes_to_file(path, data)?;
                }
            }
            Ok(())
        } else {
            Err("found none response".into())
        }
    }
}
// 定义一个自定义的证书验证器
// 实现 `ServerCertVerifier` trait，不执行任何验证。
#[derive(Debug)]
struct NoVerification;

impl rustls::client::danger::ServerCertVerifier for NoVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::CryptoProvider::get_default()
            .unwrap()
            .signature_verification_algorithms
            .supported_schemes()
            .to_vec()
    }
}

pub fn build_formdata(
    formdata: &[String],
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    let boundary = format!("----mini-curl-{}", rand::random::<u64>());
    let mut body = Vec::new();

    for item in formdata {
        if let Some((key, value)) = item.split_once('=') {
            if value.starts_with('@') {
                // 文件上传
                let filepath = &value[1..];
                let filename = std::path::Path::new(filepath)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file");
                let file_content = file_io::read_file_to_bytes(filepath)?;
                body.extend_from_slice(format!(
                    "--{}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
                    boundary, key, filename
                ).as_bytes());
                body.extend_from_slice(&file_content);
                body.extend_from_slice(b"\r\n");
            } else {
                // 普通文本字段
                body.extend_from_slice(
                    format!(
                        "--{}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n",
                        boundary, key, value
                    )
                    .as_bytes(),
                );
            }
        }
    }
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
    Ok((boundary, body))
}
