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
//TODO: remove dropping - we can lose some data in that case
//TODO: we don't need the sender - as we operate with pointers and lock anyway

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

fn read_file_char(s: &str, tx: spmc::Sender<Box<Vec<Box<Vec<u8>>>>>) -> std::io::Result<()> {
    let f = BufReader::new(File::open(s)?);
    let mut _words_counter = 0;
    let mut words_vec = Box::new(vec![]);

    for ch_vec in f.split(b' ') {
        let word = Box::new(ch_vec.unwrap().to_owned());
         
        if words_vec.len() == 100 {
            tx.send(words_vec.clone()).expect("Cannot send word to channel");
            words_vec = Box::new(vec![]);
        }
        words_vec.push(word.clone());
    }

    tx.send(words_vec.clone()).expect("Cannot send word to channel");
    drop(tx);
    Ok(())
}

fn count_words(vec_rx: spmc::Receiver<Box<Vec<Box<Vec<u8>>>>>, dict_queue: Arc<Mutex<VecDeque<Box<HashMap<String,u32>>>>>) {
    //let _ignore_chars: Vec<String> = vec!["\'", "\"", ",", ".", ";"].iter().map(|&s| s.into()).collect();
    let mut occurrences = Box::new(HashMap::new());
    'outer: loop {
        let words = vec_rx.recv();
        match words {
            Ok(mut value) => {
                for char_vec in value.iter_mut() {
                    let word = String::from(str::from_utf8(&char_vec).unwrap());
                    *occurrences.entry(word).or_insert(0) += 1;
                }
            }
            Err(spmc::RecvError) => break 'outer,
        }
    }
    dict_queue.lock().unwrap().push_back(occurrences);
    VEC_COUNT_JOBS.fetch_sub(1, Ordering::SeqCst);
}

fn collect_dict(dict_queue: Arc<Mutex<VecDeque<Box<HashMap<String,u32>>>>>) {
    //TODO: add cond var to wak them up as needed
    //TODO: test which map is bigger
    let mut first_map;
    let mut second_map;
    'outer: loop {
        {
            let mut queue = dict_queue.lock().unwrap();
            if queue.len() < 2 && VEC_COUNT_JOBS.load(Ordering::SeqCst) == 0 {
                break 'outer
            } else {
                //check for condvar wake up
                //test if enough length - if not continue outer
                if queue.len() > 1 {
                    first_map = queue.pop_front().unwrap();
                    second_map = queue.pop_front().unwrap();
                    drop(queue);
                    for (key, value) in first_map.iter_mut() {
                        *second_map.entry(key.to_string()).or_insert(*value) += 1;
                        continue;
                    }
                    dict_queue.lock().unwrap().push_back(second_map);
                }
            }
        }
    }
    return;
}

fn main() {
    let (vec_tx, vec_rx) = spmc::channel::<Box<Vec<Box<Vec<u8>>>>>();
    let dict_queue = Arc::new(Mutex::new(VecDeque::new()));
    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(3).build().unwrap();
    let config: Config = read_config();

    for _ in 0..3 {
        let vec_rx = vec_rx.clone();
        let dict_queue = dict_queue.clone();
        VEC_COUNT_JOBS.fetch_add(1, Ordering::SeqCst);
        thread_pool.spawn(move || count_words(vec_rx, dict_queue));
    }

    read_file_char(&config.read_file, vec_tx).expect("Cannot read file char by char");

   
    for _ in 0..1 {
        let dict_queue = dict_queue.clone();
        let _n = thread_pool.install(move || collect_dict(dict_queue));
    }

    println!("{}", dict_queue.lock().unwrap().len());

    for item in dict_queue.lock().unwrap().iter() {
        // println!("{:?}", item);
        println!("{:?}", item.keys().len());
        continue
    }
    // we have to pop front one element after everything finished
}
