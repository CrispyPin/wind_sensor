use std::io::{BufRead, BufReader};

const IOT_BIND_ADDR: &str = "192.168.0.108:13122";
//const HTTP_BIND_ADDR: &str = "192.168.0.108:80";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for stream in std::net::TcpListener::bind(IOT_BIND_ADDR)?.incoming().flatten() {
        println!("connected to {:?}", stream.peer_addr());
        let reader = BufReader::new(&stream);
        for l in reader.lines() {
            println!("{l:?}");
        }
        println!("done");
        // s.shutdown(std::net::Shutdown::Both).unwrap();
    }
    Ok(())
}
