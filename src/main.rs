#![feature(io)]
extern crate spmc;
extern crate serde;
extern crate serde_json;
extern crate rayon;

#[macro_use]
extern crate serde_derive;

use std::io::BufRead;
use std::sync::{Arc, Mutex, Condvar};
use std::fs::File;
use std::io::{Read, BufReader};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::collections::VecDeque;
use std::str;
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

fn count_words(vec_rx: spmc::Receiver<Arc<Mutex<Vec<Arc<Vec<u8>>>>>>, dict_queue: Arc<Mutex<VecDeque<Arc<Mutex<HashMap<Arc<Vec<u8>>,u32>>>>>>) {
    //let _ignore_chars: Vec<String> = vec!["\'", "\"", ",", ".", ";"].iter().map(|&s| s.into()).collect();
    let mut occurrences = Arc::new(Mutex::new(HashMap::new()));
    'outer: loop {
        let words = vec_rx.recv();
        match words {
            Ok(value) => {
                for word in value.lock().unwrap().iter() {
                    *occurrences.lock().unwrap().entry(word.clone()).or_insert(0) += 1;
                }
            }
            Err(spmc::RecvError) => break 'outer,
        }
    }
    dict_queue.lock().unwrap().push_back(occurrences.clone());
    VEC_COUNT_JOBS.fetch_sub(1, Ordering::SeqCst);
}

fn collect_dict(dict_queue: Arc<Mutex<VecDeque<Arc<Mutex<HashMap<Arc<Vec<u8>>,u32>>>>>>) {
    //TODO: add cond var to wak them up as needed
    //TODO: test which map is bigger
    let mut first_map = Arc::new(Mutex::new(HashMap::new()));
    let mut second_map = Arc::new(Mutex::new(HashMap::new()));
    let mut dict_queue_len;
    'outer: loop {
        {
            dict_queue_len = dict_queue.lock().unwrap().len();
        }
        if dict_queue_len < 2 && VEC_COUNT_JOBS.load(Ordering::SeqCst) == 0 {
            break 'outer
        } else {
            //check for condvar wake up
            //test if enough length - if not continue outer
            if dict_queue_len > 1 {
                println!("working here");
                let mut queue = dict_queue.lock().unwrap();
                first_map = Arc::clone(&queue.pop_front().unwrap());
                second_map = Arc::clone(&queue.pop_front().unwrap());
            }
        }
        for (key, value) in first_map.lock().unwrap().iter_mut() {
            let mut prim_map = second_map.lock().unwrap(); 
            let mut counter = *prim_map.entry(key.clone()).or_insert(*value);
            counter += *value;
        }
        dict_queue.lock().unwrap().push_back(second_map.clone());
    }
    return;
}

fn main() {
    let (vec_tx, vec_rx) = spmc::channel::<Arc<Mutex<Vec<Arc<Vec<u8>>>>>>();
    let dict_queue = Arc::new(Mutex::new(VecDeque::new()));
    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap();
    let config: Config = read_config();

    let words_vec_alarm = Condvar::new();

    for _ in 0..2 {
        let vec_rx = vec_rx.clone();
        let dict_queue = dict_queue.clone();
        VEC_COUNT_JOBS.fetch_add(1, Ordering::SeqCst);
        thread_pool.spawn(move || count_words(vec_rx, dict_queue));
    }

    read_file_char(&config.read_file, vec_tx).expect("Cannot read file char by char");

   
    for _ in 0..1 {
        let dict_queue = dict_queue.clone();
        let n = thread_pool.install(move || collect_dict(dict_queue));
    }

    let first_map = Arc::clone(&dict_queue.lock().unwrap().pop_front().unwrap());
    for (key, value) in first_map.lock().unwrap().iter_mut() {
        let sparkle_heart = str::from_utf8(&key).unwrap();
        println!("{}, {}", sparkle_heart, value);
    }
}
