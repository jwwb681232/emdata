//! Simple echo websocket server.
//! Open `http://localhost:8080/ws/index.html` in browser
//! or [python console client](https://github.com/actix/examples/blob/master/websocket/websocket-client.py)
//! could be used for testing.

use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use std::thread::spawn;
use std::{thread, time, fs};
use std::path::Path;
use std::io::{BufReader, BufRead};

extern crate redis;

use redis::{Commands};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
/// Send File Data
const FILE_DATA_INTERVAL: Duration = Duration::from_millis(250);


/// do websocket handshake and start `MyWebSocket` actor
async fn ws_index(r: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    println!("{:?}", r);
    let res = ws::start(MyWebSocket::new(), &r, stream);
    println!("{:?}", res);
    res
}

/// websocket connection is long running connection, it easier
/// to handle with an actor
struct MyWebSocket {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
}

impl Actor for MyWebSocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

/// Handler for `ws::Message`
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocket {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        // process websocket messages
        //println!("WS: {:?}", msg);

        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl MyWebSocket {
    fn new() -> Self {
        Self { hb: Instant::now() }
    }

    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });

        ctx.run_interval(FILE_DATA_INTERVAL, |_act, ctx| {
            let client = redis::Client::open("redis://127.0.0.1/").unwrap();
            let mut con = client.get_connection().unwrap();
            let res: String = con.get("my_key").unwrap();
            if res.len() > 0 {
                ctx.text(res);
                let _: () = con.set("my_key", "".to_string()).unwrap();
            }
        });
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();

    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();

    spawn(move || {
        let base_path = "./release/EmData";
        loop {
            let dirs = match fs::read_dir(&base_path) {
                Ok(dirs) => { dirs }
                Err(_) => {
                    thread::sleep(time::Duration::from_millis(500));
                    continue;
                }
            };

            if Path::new(&base_path).read_dir().map(|mut i| i.next().is_none()).unwrap_or(false) {
                thread::sleep(time::Duration::from_millis(500));
                continue;
            }

            for entry in dirs {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => {
                        thread::sleep(time::Duration::from_millis(500));
                        continue;
                    }
                };
                let path = entry.path();


                let f = fs::File::open(&path);
                let f = match f {
                    Ok(file) => { file }
                    Err(_) => {
                        thread::sleep(time::Duration::from_millis(500));
                        continue;
                    }
                };

                let mut fin = BufReader::new(f);

                let mut res = String::new();
                let _ = fin.read_line(&mut res);
                let vec: Vec<&str> = res.as_str().split(":").collect();

                println!("{}", vec[1].trim_end());

                let _: () = con.set("my_key", vec[1].trim_end()).unwrap();


                fs::remove_file(&path).unwrap();
                thread::sleep(time::Duration::from_millis(500));
            }
        }
    });

    HttpServer::new(|| {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            // websocket route
            .service(web::resource("/").route(web::get().to(ws_index)))
    })
        // start http server on 127.0.0.1:8080
        .bind("127.0.0.1:8086")?
        .run()
        .await
}