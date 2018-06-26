#![feature(io)]
extern crate spmc;
extern crate serde;
extern crate serde_json;
extern crate rayon;

#[macro_use]
extern crate serde_derive;
extern crate chan;

use chan::{Receiver, Sender};

use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::thread;
use std::io::{Read, BufReader};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

//TODO: add condvar to wake up threads

#[derive(Serialize, Deserialize)]
struct Config {
    threads: u16,
    read_file: String,
    alph_count: String,
    nume_count: String,
}

static VEC_COUNT_JOBS: AtomicUsize = ATOMIC_USIZE_INIT;

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

fn count_words(vec_rx: spmc::Receiver<Arc<Mutex<Vec<Arc<Vec<u8>>>>>>, dict_tx: chan::Sender<Arc<Mutex<HashMap<Arc<Vec<u8>>, u32>>>>) {
    let mut occurrences = Arc::new(Mutex::new(HashMap::new()));
    'outer: loop {
        let words = vec_rx.try_recv();
        match words {
            Ok(value) => {
                for word in value.lock().unwrap().iter() {
                    *occurrences.lock().unwrap().entry(word.clone()).or_insert(0) += 1;
                }
            }
            Err(spmc::TryRecvError::Empty) => continue,
            Err(spmc::TryRecvError::Disconnected) => break 'outer,
        }
    }
}

fn collect_dict(dict_tx: chan::Receiver<Arc<Mutex<HashMap<Arc<Vec<u8>>, u32>>>>) {
    //TODO: mark them and if already have one - wake up this
    'outer: loop {
        let first_map = dict_tx.recv();
        let second_map = dict_tx.recv();
    }
}

fn main() {
    let config: Config = read_config();

    let (vec_tx, vec_rx) = spmc::channel::<Arc<Mutex<Vec<Arc<Vec<u8>>>>>>();
    let (dict_tx, dict_rx) = chan::sync::<Arc<Mutex<HashMap<Arc<Vec<u8>>, u32>>>>(0);

    //let mut thread_pool = Vec::new();

    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(8).build().unwrap();

    for _ in 0..2 {
        let vec_rx = vec_rx.clone();
        let dict_tx = dict_tx.clone();
        VEC_COUNT_JOBS.fetch_add(1, Ordering::SeqCst);
        thread_pool.install(move || count_words(vec_rx, dict_tx));
//        thread_pool.push(thread::spawn(move || count_words(vec_rx, dict_tx) ));
    }
    
    read_file_char(&config.read_file, vec_tx).expect("Cannot read file char by char");

    println!("{}", config.read_file);
   
    for _ in 0..1 {
        let dict_rx = dict_rx.clone();
        let sum_dict = thread_pool.install(move || collect_dict(dict_rx));
    }
}
