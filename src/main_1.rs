#![feature(io)]
extern crate spmc;
extern crate serde;
extern crate serde_json;

use std::str::Chars;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::thread;
use std::string;
use std::io::{self, Read, BufReader};
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

//type Chunk = (Arc<Mutex<Vec<str>>>, usize, usize);

fn read_config() -> Config {
    // TODO: add exception handling for the file read
    let mut file = File::open("config.json").unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();
    let config: Config = serde_json::from_str(&mut data).expect("Couldn't read config");
    return config;
}

fn read_file_char(s: &str, tx: spmc::Sender<Box<Vec<String>>>) -> std::io::Result<()> {
    let mut file = File::open(s)?;
    let mut f = BufReader::new(file);

    let ignore_chars: Vec<String> = vec!["\'", "\"", ",", ".", ";"].iter().map(|&s| s.into()).collect();

    let mut _words_counter = 0;
    let mut words_vec = Box::new(Vec::new());
    let mut word: String = "".to_string();

    tx.send(Box::new(vec!["Yeaah".to_string()]));

    for ch_ptr in f.chars() {
        let ch: String = ch_ptr.unwrap().to_string();

        if word.len() == 0 && ignore_chars.contains(&ch) {
            println!("Ignoring");
            continue;
        } else if ch == " " || ch == "\n" {
            tx.send(Box::new(vec![word.to_owned()]));
            word.clear();
        } else {
            word.push_str(&ch);
        }
        
        if _words_counter == 100 {
            _words_counter = 0;
            tx.send(words_vec);
            let mut words_vec: Vec<String>;
        }

    }
    println!("{}", word); 
    //println!("Char: {:?}", count);
    drop(tx);
    Ok(())
}

fn main() {
    let config: Config = read_config();

    let (tx, rx) = spmc::channel();
    let mut thread_pool = Vec::new();

    for _n in 0..5 {
        let rx = rx.clone();
        thread_pool.push(thread::spawn(move || {
            //let mut words: Vec<String> = rx.recv().unwrap();
            loop {
                let words = rx.try_recv();
                match words {
                    Ok(value) => println!("{:?}", value),
                    Err(_e) => continue,
                }
            }
            //println!("worker {} recvd: {:?}", n,  words);
        }));
    }
    
    read_file_char(&config.read_file, tx);
    
    println!("{}", config.read_file);
    
    //tx.send(vec!["Hello".to_string()]);
    //drop(tx);
}
