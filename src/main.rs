#![feature(io)]
extern crate spmc;
extern crate serde;
extern crate serde_json;
extern crate rayon;
extern crate crossbeam;
extern crate crossbeam_channel;

#[macro_use]
extern crate serde_derive;

use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::{Read, BufReader, Write};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::str;

//TODO: add exception handling for the file read
//TODO: no fastest way to clear string from chars - need to fix this
//TODO: test which map is bigger and iterate over smaller - will save time
//TODO: reimplement checking of empty words to somewhere else
//TODO: not clear constant config files

#[derive(Serialize, Deserialize)]
struct Config {
    threads: usize,
    count_jobs: u16,
    merge_jobs: u16,
    read_file: String,
    alph_count: String,
    nume_count: String,
}

fn read_config() -> Config {
    let mut file = File::open("config.json").expect("Cannot find config file");
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();
    let config: Config = serde_json::from_str(&mut data)
                            .expect("Couldn't read config");
    return config;
}

fn read_file_char(s: &str, tx: spmc::Sender<Box<Vec<Box<Vec<u8>>>>>) 
                -> std::io::Result<()> {
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

fn count_words(vec_rx: spmc::Receiver<Box<Vec<Box<Vec<u8>>>>>, 
                dict_queue: Arc<Mutex<VecDeque<Box<HashMap<String,u32>>>>>,
                wake_tx: crossbeam_channel::Sender<bool>) {
    let mut occurrences = Box::new(HashMap::new());
    let patterns : &[_] = &[    '\"', '\'', '+', '-',
                                '(', ')', '.', '_', '*',
                                '#', '/', '%', '^', '˙', '´',
                                '─', '-', ';', ' ', '—', 
                                ',', '"', '′', '″', '˝', '…', '\\',
                                '┼', '┬', '┴', '†', '°', '–',
                                '┤', '┘', '└', '┌', '┐', '├',
                                '$', '&', '[', ']', '│', '|',
                                '”', '“', '’', '‘', '„', '!', '=',
                                '-', '∴', ':' , '?', '{', '}',
                                'ᵇ', 'ᵈ', 'ᵐ', 'ᶜ', 'ʰ', 'ª',
                                '«', '»',
                                '1', '2', '3', '4', '5',
                                '6', '7', '8', '9', '0'];
    'outer: loop {
        let words = vec_rx.recv();
        match words {
            Ok(mut value) => {
                for char_vec in value.iter_mut() {
                    // we need to take string, split by \n, trim all chars and lowercase it
                    // not the fastest - but we use std for the rescue
                    // also not sure if split by \n here
                    let mut raw_words = String::from(str::from_utf8(&char_vec).unwrap());
                    for raw_word in raw_words.split("\n") {
                        let word = raw_word.trim_matches(patterns)
                                        .to_lowercase();
                        *occurrences.entry(word).or_insert(0) += 1;
                    }

                }
            }
            Err(spmc::RecvError) => break 'outer,
        }
    }
    dict_queue.lock().unwrap().push_back(occurrences);
    //we notify our collect dictionary that we pushed and close channel after
    wake_tx.send(true);
    drop(wake_tx);
}

fn collect_dict(dict_queue: Arc<Mutex<VecDeque<Box<HashMap<String,u32>>>>>,
                wake_rx: crossbeam_channel::Receiver<bool>,
                finish_tx: crossbeam_channel::Sender<bool>) {
    let mut first_map;
    let mut second_map;
    let mut finish_reading = false;
    'outer: loop {
        {
            //our recv is blocking operation - it will wait for recv
            //if all sender closed - return none
            wake_rx.recv();
            match wake_rx.recv() {
                Some(_) => (),
                None => finish_reading = true,
            };
            // we need to check before starting merging - if no words counter
            // and length 1 - we can stop merging and break outer
            let mut queue = dict_queue.lock().unwrap();
            if queue.len() > 1 {
                first_map = queue.pop_front().unwrap();
                second_map = queue.pop_front().unwrap();
                drop(queue);
                for (key, value) in first_map.iter_mut() {
                    *second_map.entry(key.to_string()).or_insert(0) += *value;
                    continue;
                }
                dict_queue.lock().unwrap().push_back(second_map);
            } else if queue.len() <= 1 && finish_reading {
                finish_tx.send(true);
                break 'outer;
            }
        }
    }
}

fn write_vec_tuple(tuple_vec: &Vec<(&String, &u32)>, filename: &String) {
    let mut out_file = File::create(filename)
                        .expect("Unable to open file to write");
    for el in tuple_vec.iter() {
        if el.0 != "" {
            out_file.write_fmt(format_args!("{}:{}\n", el.0, el.1))
                        .expect("Can't write to file");
        }
    }
}

fn main() {
    // vec_* -> send pointers to vectors from reader to counters
    // wake_* -> wake up mergers as we parsed vector
    // finish_* -> let know main thread that we finished merging
    let (vec_tx, vec_rx) = spmc::channel::<Box<Vec<Box<Vec<u8>>>>>();
    let (wake_tx, wake_rx) = crossbeam_channel::unbounded();
    let (finish_tx, finish_rx) = crossbeam_channel::unbounded();

    // we need to be able to recieve at once 2 dictionary
    // that's why vecdeque
    let dict_queue = Arc::new(Mutex::new(VecDeque::new()));
    let config: Config = read_config();
    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(config.threads)
                        .build().unwrap();

    for _ in 0..config.count_jobs {
        let vec_rx = vec_rx.clone();
        let wake_tx = wake_tx.clone();
        let dict_queue = dict_queue.clone();

        thread_pool.spawn(move || count_words(vec_rx, dict_queue, wake_tx));
    }

    //we have extra copy of sender here - need to close it aswell
    drop(wake_tx);

    read_file_char(&config.read_file, vec_tx)
        .expect("Cannot read file char by char");
   
    for _ in 0..config.merge_jobs {
        let dict_queue = dict_queue.clone();
        let wake_rx = wake_rx.clone();
        let finish_tx = finish_tx.clone();

        thread_pool.spawn(move || collect_dict(dict_queue, 
                                                wake_rx, finish_tx));
    }

    drop(finish_tx);

    // block until finished merging all maps
    loop  {
        match finish_rx.recv() {
            Some(_) => continue,
            None => break,
        }
    }
    drop(thread_pool);

    let res_dict = dict_queue.lock().unwrap().pop_front();
    match res_dict {
        Some(v) => {
            let mut count_vec: Vec<(&String, &u32)> = v.iter().collect();
            count_vec.sort_by(|a, b| b.1.cmp(a.1));
            write_vec_tuple(&count_vec, &config.nume_count);
            count_vec.sort_by(|a, b| a.0.cmp(b.0));
            write_vec_tuple(&count_vec, &config.alph_count);
        },
        None => (),
    }
    
}