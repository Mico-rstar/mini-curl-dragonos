use clap::Parser;
use url::Url;

mod file_io;
mod parser;
mod requester;
mod response;
mod structs;

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

    #[arg(
        short = 'o',
        long = "output",
        value_name("STRING"),
        num_args = 1,
        help = "Set output file for response"
    )]
    output: Option<String>,

    #[arg(
        short = 'F',
        long = "formdata",
        value_name = "STRING",
        num_args = 1..,
        help = "Set form data for multipart/form-data, e.g. -F key=value"
    )]
    formdata: Vec<String>,

    #[arg(required = true)]
    url: String,
}

fn main() {
    // 定义命令行界面
    let args = Cli::parse();
    let url_str = args.url;
    let url = Url::parse(&url_str).unwrap();
    let mut request = requester::Request::new(&url);

    
    if let Some(data) = args.data {
        request.set_data(&data);
    }
    if !args.formdata.is_empty() {
        let (boundary, body) = requester::build_formdata(&args.formdata).unwrap();
        request.set_formdata(&body, boundary);
    }
    if let Some(header) = args.header {
        request.set_header(&header);
    }
    
    // 默认为GET
    let method = args.method;
    let scheme = url.scheme();

    if scheme == "https" {
        if let Err(e) = request.https_do(method) {
            eprintln!("Error during HTTP request: {}", e);
            return; // Exit if there's an error
        }
    } else if scheme == "http" {
        if let Err(e) = request.http_do(method) {
            eprintln!("Error during HTTP request: {}", e);
            return; // Exit if there's an error
        }
    } else {
        println!("unsupported scheme: {}", scheme);
    }

    if let Some(output) = args.output {
        if let Err(e) = request.response_output(&output) {
            eprintln!("Error during write response to output: {}", e);
            return;
        }
    }
}
