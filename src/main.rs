#![feature(io)]
extern crate spmc;
extern crate serde;
extern crate serde_json;

use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::thread;
use std::io::{Read, BufReader};

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

fn read_file_char(s: &str, tx: spmc::Sender<Arc<Mutex<Vec<Arc<Vec<u8>>>>>>) -> std::io::Result<()> {
    let f = BufReader::new(File::open(s)?);

    let _ignore_chars: Vec<String> = vec!["\'", "\"", ",", ".", ";"].iter().map(|&s| s.into()).collect();

    let mut _words_counter = 0;
    let mut words_vec = Arc::new(Mutex::new(vec![]));

    for ch_vec in f.split(b' ') {
        let word = Arc::new(ch_vec.unwrap().to_owned());
        
        if words_vec.lock().unwrap().len() == 100 {
            tx.send(words_vec.clone()).expect("Cannot send word to channel");
            words_vec = Arc::new(Mutex::new(vec![]));
        }
        words_vec.lock().unwrap().push(word.clone());
    }

    tx.send(words_vec.clone()).expect("Cannot send word to channel");

    drop(tx);
    Ok(())
}

fn main() {
    let config: Config = read_config();

    let (tx, rx) = spmc::channel::<Arc<Mutex<Vec<Arc<Vec<u8>>>>>>();
    let mut thread_pool = Vec::new();

    for _n in 0..5 {
        let rx = rx.clone();
        thread_pool.push(thread::spawn(move || {
            //let mut words: Vec<String> = rx.recv().unwrap();
            loop {
                let words = rx.try_recv();
                match words {
                    Ok(value) => println!("{}", value.lock().unwrap().len()),
                    Err(_e) => continue,
                }
            }
            //println!("worker {} recvd: {:?}", n,  words);
        }));
    }
    
    read_file_char(&config.read_file, tx).expect("Cannot read file char by char");
    
    println!("{}", config.read_file);
    
    //tx.send(vec!["Hello".to_string()]);
    //drop(tx);
}
