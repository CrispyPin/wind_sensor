use std::{
    io::{BufRead, BufReader},
    sync::mpsc,
    thread::{self, yield_now},
    time::Duration,
};

const IOT_BIND_ADDR: &str = "0.0.0.0:13122";
const TIMEOUT_SECONDS: u64 = 15;
//const HTTP_BIND_ADDR: &str = "192.168.0.108:80";

fn main() {
    let (pipe_in, pipe_out) = mpsc::channel();
    let _sensor_data_thread = thread::spawn(move || {
        println!("listening on {IOT_BIND_ADDR} for connection from pico");
        for stream in std::net::TcpListener::bind(IOT_BIND_ADDR)
            .unwrap()
            .incoming()
            .flatten()
        {
            println!("connected to {:?}", stream.peer_addr());
            let reader = BufReader::new(&stream);
            stream
                .set_read_timeout(Some(Duration::from_secs(TIMEOUT_SECONDS)))
                .unwrap();
            println!("{:?}", stream.read_timeout());
            for l in reader.lines() {
                match l {
                    Ok(data) => {
                        println!("{data}");
                        pipe_in.send(data).unwrap()
                    }
                    Err(_) => break,
                }
            }
            println!("disconnected");
        }
    });
    loop {
        // todo save data
        yield_now();
    }
}
