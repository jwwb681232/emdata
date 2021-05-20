use std::fs;
use std::io::{BufReader, BufRead};
use std::{thread, time};
use std::path::Path;
use std::{net::TcpListener, thread::spawn};
use tungstenite::{accept_hdr, handshake::server::{Request, Response}, Message, accept};

extern crate redis;

use redis::{Commands};

fn main() {
    let client = match redis::Client::open("redis://127.0.0.1/") {
        Ok(c) => c,
        Err(_) => {
            println!("{}", "Please run redis-server first and try again");
            loop { thread::sleep(time::Duration::from_millis(10000)); }
        }
    };

    let mut con = match client.get_connection() {
        Ok(c) => c,
        Err(_) => {
            println!("{}", "Please run redis-server first and try again");
            loop { thread::sleep(time::Duration::from_millis(10000)); }
        }
    };

    spawn(move || {
        let base_path = "./release/EmData";
        loop {
            let dirs = match fs::read_dir(&base_path) {
                Ok(dirs) => { dirs }
                Err(_) => {
                    thread::sleep(time::Duration::from_millis(100));
                    continue;
                }
            };

            if Path::new(&base_path).read_dir().map(|mut i| i.next().is_none()).unwrap_or(false) {
                thread::sleep(time::Duration::from_millis(100));
                continue;
            }

            for entry in dirs {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => {
                        thread::sleep(time::Duration::from_millis(100));
                        continue;
                    }
                };
                let path = entry.path();


                let f = fs::File::open(&path);
                let f = match f {
                    Ok(file) => { file }
                    Err(_) => {
                        thread::sleep(time::Duration::from_millis(100));
                        continue;
                    }
                };

                let mut fin = BufReader::new(f);

                let mut res = String::new();
                let _ = fin.read_line(&mut res);
                let vec: Vec<&str> = res.as_str().split(":").collect();

                println!("{}", vec[1].trim_end());

                let _: () = con.set("my_key", vec[1].trim_end()).unwrap();


                //fs::remove_file(&path).unwrap();
                thread::sleep(time::Duration::from_millis(100));
            }
        }
    });

    let server = TcpListener::bind("127.0.0.1:8086").unwrap();
    for stream in server.incoming() {
        let mut con = client.get_connection().unwrap();
        spawn (move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                let msg = websocket.read_message().unwrap();
                let res: String = con.get("my_key").unwrap();
                let _: () = con.set("my_key", "".to_string()).unwrap();


                if res.len() > 0 {
                    let _ = websocket.write_message(Message::Text(res));
                    let _: () = con.set("my_key", "".to_string()).unwrap();
                }

                thread::sleep(time::Duration::from_millis(50));
                /*let res: String = con.get("my_key").unwrap();
                if res.len() > 0 {
                    let _ = websocket.write_message(Message::Text(res));
                    let _: () = con.set("my_key", "".to_string()).unwrap();
                }*/
            }
        });
    }
    /*for stream in server.incoming() {
        let mut con = client.get_connection().unwrap();
        spawn(move || {
            let mut websocket = accept_hdr(stream.unwrap(), |_req: &Request, response: Response| {
                Ok(response)
            }).unwrap();

            loop {

                let msg = websocket.read_message().unwrap();
                let res: String = con.get("my_key").unwrap();
                if msg.is_binary() || msg.is_text() || res.len() > 0 {
                    if res.len() > 0 {
                        let _ = websocket.write_message(Message::Text(res));
                        let _: () = con.set("my_key", "".to_string()).unwrap();
                    }
                    let _ = websocket.write_message(msg).unwrap();
                }




                //thread::sleep(time::Duration::from_millis(50));
            }
        });
    }*/

    loop {}
}