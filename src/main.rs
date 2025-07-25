use clap::Parser;
use url::{Host, Url};

mod parser;
mod requester;

#[derive(Parser)]
#[command(name = "mini-curl", version = "1.0", about = "A curl-like tool")]
struct Cli {
    #[arg(
        short = 'X',
        long = "request",
        default_value = "GET",
        value_name("STRING"),
        num_args = 1,
        help = "Set request method"
    )]
    method: String,

    #[arg(
        short = 'H',
        long = "header",
        value_name("STRING"),
        num_args = 1,
        help = "Set request header"
    )]
    header: Option<String>,

    #[arg(
        short = 'd',
        long = "data",
        value_name("STRING"),
        num_args = 1,
        help = "Set data for request"
    )]
    data: Option<String>,

    #[arg(required = true)]
    url: String,
}

fn main() {
    // 定义命令行界面
    let args = Cli::parse();

    let url_str = args.url;

    let url = Url::parse(&url_str).unwrap();

    let mut request = requester::request::new(&url);

    if let Some(header) = args.header {
        request.set_header(&header);
    }

    if let Some(data) = args.data {
        request.set_data(&data);
    }

    // 默认为GET
    let method = args.method;
    let scheme = url.scheme();

    if scheme == "https" {
        request.https_do(method);
    } else if scheme == "http" {
        if let Err(e) = request.http_do(method) {
            eprintln!("Error during HTTP request: {}", e);
            return; // Exit if there's an error
        } 
    } else {
        println!("unsupported scheme: {}", scheme);
    }
}
