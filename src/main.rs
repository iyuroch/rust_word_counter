use std::sync::mpsc::channel;
use std::thread;
use std::fs::File;
use std::io::Read;

extern crate serde;
extern crate serde_json;
use serde_json::{Value, Error};

#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize)]
struct Config {
    threads: u16,
    read_file: String,
    alph_count: String,
    nume_count: String,
}

fn read_config() -> Config {
    // TODO: add exception handling for the file read
    let mut file = File::open("config.json").unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();
    let config: Config = serde_json::from_str(&mut data).expect("Couldn't read config");
    return config;
}

fn main() {
    let config: Config = read_config();
    let (sender, receiver)
    //let (sender, receiver) = channel();
    //thread::spawn(move|| {
    //    sender.send("Hello!");
    //});
    //// just something here
    //println!("{:?}", receiver.recv().unwrap());
    //println!("Hello, world!");
    println!("{}", config.threads);
}
