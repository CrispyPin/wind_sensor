use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    sync::mpsc,
    thread,
    time::{Duration, SystemTime},
};

const IOT_BIND_ADDR: &str = "0.0.0.0:13122";
const TIMEOUT_SECONDS: u64 = 15;
const HTTP_BIND_ADDR: &str = "127.0.0.1:8069";
const DATA_FILE_PATH: &str = "saved.txt";
const HTML_TEMPLATE_PATH: &str = "template.html";
const HTML_REPLACE_TOKEN: &str = "CONTENT_HERE";
type Batch = (u128, Vec<u8>);

fn main() {
    // todo read old data
    let mut all_data = Vec::new();
    let (pipe_in, pipe_out) = mpsc::channel();
    let _sensor_data_thread = thread::spawn(move || {
        println!("listening on {IOT_BIND_ADDR} for connection from pico");
        for stream in std::net::TcpListener::bind(IOT_BIND_ADDR)
            .unwrap()
            .incoming()
            .flatten()
        {
            println!("connected to {:?}", stream.peer_addr());
            let mut out_file = File::options().append(true).open(DATA_FILE_PATH).unwrap();
            let reader = BufReader::new(&stream);
            stream
                .set_read_timeout(Some(Duration::from_secs(TIMEOUT_SECONDS)))
                .unwrap();
            println!("{:?}", stream.read_timeout());
            for l in reader.lines() {
                match l {
                    Ok(msg) => {
                        // println!("{msg}");
                        if let Some((time, data)) = parse_pico_message(&msg) {
                            let mut text = format!("{time}:");
                            for &d in &data {
                                text.push((d + b'0') as char)
                            }
                            text.push('\n');
                            out_file.write_all(text.as_bytes()).unwrap();
                            pipe_in.send((time, data)).unwrap();
                        } else {
                            println!("malformed message: {msg}");
                        }
                    }
                    Err(_) => break,
                }
            }
            println!("disconnected");
        }
    });

    println!("listening on {HTTP_BIND_ADDR} for HTTP request");
    for mut stream in std::net::TcpListener::bind(HTTP_BIND_ADDR)
        .unwrap()
        .incoming()
        .flatten()
    {
        // we don't even need to read the request, the same thing is always sent back
        for batch in pipe_out.try_iter() {
            all_data.push(batch);
        }

        let generated_table = generate_visualisation(&all_data);

        let html = fs::read_to_string(HTML_TEMPLATE_PATH)
            .unwrap()
            .replace(HTML_REPLACE_TOKEN, &generated_table);
        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            html.len(),
            html
        );
        println!("{:?}", stream.write_all(http_response.as_bytes()));
    }
}

fn generate_visualisation(all_data: &[Batch]) -> String {
    //TODO
    String::new()
}

fn parse_pico_message(msg: &str) -> Option<Batch> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let values = msg.split_once(':')?.1;
    let values = values.chars().map(|c| c as u8 - b'0').collect();

    Some((timestamp, values))
}
