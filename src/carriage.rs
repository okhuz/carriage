use std::io::prelude::*;
use std::net::TcpStream;
use std::net::TcpListener;
use std::clone::Clone;

pub mod router;
pub mod method;
pub mod route;
pub mod request;
pub mod response;
pub mod thread_pool;
pub mod codes;

use request::*;

pub struct Carriage<'a>
{
    listener: TcpListener,
    pub router: &'a mut router::Router<'a>
}

impl<'a> Carriage <'a>
{
    pub fn new(address: &str, port: &str, router: &'a mut router::Router<'a>) -> Carriage<'a> {
        let mut ip: String = address.to_owned();
        let port: String = port.to_owned();
        ip.push_str(":");
        ip.push_str(&port);
        let listener = match TcpListener::bind(ip) {
            Ok(listener) => listener,
            Err(e) => {
                println!("Problem parsing arguments: {}", e);
                std::process::exit(1);
            }
        };
        Carriage {
            listener,
            router
        }
    }

    pub fn connect(&'a mut self) {
        for stream in self.listener.incoming() {
            let f = self.router.clone();
            let stream = stream.unwrap();
            handle_request(f, stream);
        }
    }
}

fn get_method<'a>(request: Option<&'a str>) -> Result<String, String> {
    match request {
        Some(request) => {
            let x = &request.find(" ");
            let slice = match x {
                Some(x) => &request[0..*x],
                None => { "" }
            };
            return Ok(slice.to_string());
        },
        None => {
            Err("No request".to_string())
        }
    }
}

fn decide_method(method_string: &str) -> method::Method {
    match method_string {
        "GET" => method::Method::GET,
        "POST" => method::Method::POST,
        "PUT" => method::Method::PUT,
        "PATCH" => method::Method::PATCH,
        "DELETE" => method::Method::DELETE,
        "OPTIONS" => method::Method::OPTIONS,
        "HEAD" => method::Method::HEAD,
        _ => method::Method::NONE
    }
}

fn get_url<'a>(request: Option<&'a str>) -> Result<String, String> {
    match request {
        Some(request) => {
            let first_result = request.find(" ").unwrap_or(0);
            let sliced_first = &request[first_result+1..];
            let second_result = sliced_first.find(" ").unwrap_or(0);
            let final_slice = &sliced_first[..second_result];
            return Ok(final_slice.to_string());
        },
        None => {
            Err("No request".to_string())
        }
    }
}


fn get_body<'a> (request: &'a str) -> Result<SimpleBodyData, String>
{
    let double_space_index = request.find("\r\n\r\n").unwrap_or(0);
    let sliced_first = &request[double_space_index + 4..];
    let second_result = sliced_first.find("\u{0}").unwrap_or(0);
    let final_slice = &sliced_first[..second_result];
    let deserialized: SimpleBodyData = SimpleBodyData::new(serde_json::from_str(final_slice).unwrap());
    Ok(deserialized)
}

fn handle_request(router: router::Router, mut stream: TcpStream)
{
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..]);
    let url = get_url(Some(&request[..]));
    let method = match get_method(Some(&request[..])) {
        Ok(m) => {decide_method(&m)},
        Err(_e) => { method::Method::NONE }
    };

    let res = match url {
        Ok(url) => {
            let body = get_body(&request).unwrap();
            let request = Request::new(&url, &method, body);
            let response = router.check_routes(request);
            response
        },
        Err(e) => {
            println!("{}", e);
            response::Response { code: codes::HttpCodes::NotFound, body: "{\"error\": \"No responser\"}" }
        }
    };

    let response = format!(
        "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        res.code,
        res.body.len(),
        res.body
    );

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
