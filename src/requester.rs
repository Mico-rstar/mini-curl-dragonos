use crate::response::{self, Response};
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

#[derive(Copy, Clone)]
enum Method {
    GET,
    POST,
}

pub struct Request {
    data: Option<String>,
    header: Option<String>,
    url: url::Url,
    response: Option<Response>,
}

impl Request {
    pub fn new(_url: &url::Url) -> Self {
        Request {
            data: None,
            header: None,
            url: _url.clone(),
            response: None,
        }
    }

    fn construct_header(&mut self, method: Method) {
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

        // data length
        let dl = self.data.clone().unwrap_or_default().len();

        match method {
            Method::GET => {
                self.header = Some(format!(
                    "GET {}{} HTTP/1.1\r\n\
                    Host: {}\r\n\
                    Content-Type: application/x-www-form-urlencoded\r\n\
                    Content-Length: {}\r\n\
                    Connection: close\r\n\r\n\
                    ",
                    path, query, host, dl
                ));
            }
            Method::POST => {
                self.header = Some(format!(
                    "POST {} HTTP/1.1\r\n\
                Host: {}\r\n\
                Content-Type: application/x-www-form-urlencoded\r\n\
                Content-Length: {}\r\n\
                Connection: close\r\n\r\n\
                ",
                    path, host, dl
                ));
            }
        }
    }

    fn match_method(method_str: String) -> Result<Method, Box<dyn std::error::Error>> {
        match method_str.as_str() {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            _ => Err("unsupported method".into()),
        }
    }

    pub fn set_header(&mut self, _h: &String) -> &mut Self {
        self.header = Some(_h.clone());
        self
    }
    pub fn set_data(&mut self, _d: &String) -> &mut Self {
        self.data = Some(_d.clone());
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
        self.construct_header(Method::GET);
        let request = &self.header.clone().unwrap_or_default();

        // 发送请求
        stream.write_all(request.as_bytes())?;

        // // 创建一个缓冲区来接收数据
        // let mut buffer = String::new();

        // // 读取服务器返回的数据
        // stream.read_to_string(&mut buffer)?;
        // // Ok(buffer)
        // println!("buffer: {}", buffer);

        let raw = Response::read(&mut stream)?;
        self.response = Some(Response::parse(raw));
        if let Some(resq) = &self.response {
            match &resq.body {
                response::ResponseBody::Text(text) => {
                    println!("{}", text);
                }
                response::ResponseBody::Binary(data) => {
                    println!("Warning: Binary output can mess up your terminal.");
                    println!("Warning: or consider '--output <FILE>' to save to a file.");
                }
            }
        }
        

        Ok(())
    }

    pub fn post(&mut self, addrs: &Vec<SocketAddr>) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(&addrs[..])?;

        // 构造完整的请求头
        self.construct_header(Method::POST);
        let request =
            self.header.clone().unwrap_or_default() + &self.data.clone().unwrap_or_default();
        // println!("request: {}", request);

        // 发送请求
        stream.write_all(request.as_bytes())?;

        // // 创建一个缓冲区来接收数据
        // let mut buffer = String::new();

        // // 读取服务器返回的数据
        // stream.read_to_string(&mut buffer)?;
        // Ok(buffer)

        let raw = Response::read(&mut stream)?;
        self.response = Some(Response::parse(raw));

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
            Method::GET => request = self.header.clone().unwrap_or_default(),
            Method::POST => {
                request =
                    self.header.clone().unwrap_or_default() + &self.data.clone().unwrap_or_default()
            }
        }

        tls_stream.write_all(request.as_bytes())?;
        tls_stream.flush()?;

        let raw = Response::read(&mut tls_stream)?;
        self.response = Some(Response::parse(raw));

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


