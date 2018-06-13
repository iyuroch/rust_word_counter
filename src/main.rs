extern crate spmc;
extern crate serde;
extern crate serde_json;

use std::fs::File;
use std::thread;
use std::string;
use std::io::{self, Read};
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

fn filename_to_string(s: &str) -> io::Result<String> {
    let mut file = File::open(s)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?
    Ok(s)
}

fn words_by_line<'a>(s: &'a str) -> Vec<&'a str> {
    let chars: &[_] = &[',', '"', '.', '!', '!', '(', ')'];
    return s.split_whitespace()
        .map(|x| x.trim_matches(chars))//trim_matches(chars))
        .collect()
}

fn main() {
    let config: Config = read_config();

    let (tx, rx) = spmc::channel();
    let mut thread_pool = Vec::new();

    for n in 0..5 {
        let rx = rx.clone();
        thread_pool.push(thread::spawn(move || {
            let words = rx.recv().unwrap();

            //let mut frequency: HashMap<&str, u32> = HashMap::new();
            //for word in &words { // word is a &str
            //    *frequency.entry(word.to_lowercase()).or_insert(0) += 1;
            //}
            //println!("{:?}", frequency);
            println!("worker {} recvd: {:?}", n, words);
        }));
    }

    let whole_file = filename_to_string(&config.read_file).unwrap();
    let words = words_by_line(&whole_file);

    for chunk in words.chunks(100) {
        tx.send("Hi").unwrap();
        //tx.send(chunk.to_owned()).unwrap();
        println!("len: {}", chunk.len());
        //println!("{:?}", chunk);
    }
    drop(tx);

    for thread_ in thread_pool {
      thread_.join().unwrap();
    }

    println!("{}", config.threads);
}
