#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
mod lib;
extern crate rouille;
use std::thread;
use std::result::Result;
use rouille::Response;
use lib::parse_url::ParsedUrl;


fn main() {
    let rocket_thread = thread::spawn(move || {
        rocket_main();
    });
    let rouille_thread = thread::spawn(move || {
        rouille_main();
    });
    rocket_thread.join().unwrap();
    rouille_thread.join().unwrap();
}

fn rocket_main() {
    let conf = rocket::config::Config::build(rocket::config::Environment::Development)
        .address("0.0.0.0")
        .port(10098)   
        .unwrap();
    rocket::custom(conf).mount("/", routes![]).launch();
}

fn rouille_main() {
    rouille::start_server("0.0.0.0:10099", move |request| {
        println!("{:?}", request);
        let url = &request.raw_url();

        let qur = &request.get_param("arg1");
        println!("{}", url);
        println!("{:?}", qur);
        let parsed_url = parse_url(url);
        println!("{:?}", parsed_url);
        return Response::text(format!("{:#?}", parsed_url));
    })
}

fn decode_url(url: &str) -> Result<String, std::str::Utf8Error> {
    Ok(String::from(percent_encoding::percent_decode_str(url).decode_utf8()?))
}

fn parse_url(raw_url: &str) -> Result<ParsedUrl, std::str::Utf8Error> {
    ParsedUrl::parse_new(raw_url, decode_url)
}

#[test]
fn t() {}