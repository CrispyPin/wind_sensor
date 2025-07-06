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
const BAR_BG: &str = "#222";
const BAR_FG: &str = "#93e";
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
            let mut out_file = File::options()
                .append(true)
                .create(true)
                .open(DATA_FILE_PATH)
                .unwrap();
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
    let mut sums = [0; 8];
    let mut total_points = 0;
    for batch in all_data {
        for &d in &batch.1 {
            sums[d as usize] += 1;
            total_points += 1;
        }
    }
    let mut percentages = [0.0; 8];
    if total_points > 0 {
        for d in 0..8 {
            let p = sums[d] as f64 / total_points as f64;
            percentages[d] = p * 100.;
        }
    }

    let mut out = String::from("<table><tr class=\"bars\">\n");
    for d in 0..8 {
        out.push_str(&format!(
            "<td style=\"background: linear-gradient({1} {0:.2}%, {2} {0:.2}%);\"></td>\n",
            100. - percentages[d],
            BAR_BG,
            BAR_FG
        ));
    }
    out.push_str("</tr><tr>\n");
    for d in 0..8 {
        out.push_str(&format!("<td>{d}</td>\n"));
    }
    out.push_str("</tr><tr>\n");
    for d in 0..8 {
        out.push_str(&format!("<td>{:.1}%</td>\n", percentages[d]));
    }
    out.push_str("</tr></table>\n");

    if let Some((time, data)) = all_data.last() {
        out.push_str(&format!(
            "<p>last known direction: {} at {} (UTC)</p>",
            data.last().unwrap_or(&0),
            formatted_time((time / 1000) as u64)
        ));
    }

    out
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

fn formatted_time(unix_time: u64) -> String {
    // i wrote this ages ago and don't dare touch it :)
    let second = unix_time % 60;
    let minute = unix_time / 60 % 60;
    let hour = unix_time / 3600 % 24;

    let days_since_epoch = unix_time / (3600 * 24);
    let years_since_epoch = (days_since_epoch * 400) / 146097;
    // 365.2425 days per year
    /*
    days = years * 365 + years/4 + years/400 - years/100
    d = y*365 + y/4 + y/400 - y/100
    d = (365y*400)/400 + 100y/400 + y/400 - 4y/400
    d*400 = (365y*400) + 100y + y - 4y
    d*400 = 400*365*y + 97*y
    d*400 = y* (400*365 + 97)
    d*400 = y*146097
    years = (days * 400) / 146097
    */
    let year = years_since_epoch + 1970;

    let is_leap_year = (year % 4 == 0) && !((year % 100 == 0) && !(year % 400 == 0));
    let feb = if is_leap_year { 29 } else { 28 };
    let month_lengths = [31, feb, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let leap_days = years_since_epoch / 4;
    let mut day = days_since_epoch - leap_days - years_since_epoch * 365;
    let mut month = 0;
    for i in 0..12 {
        if day < month_lengths[i] {
            month = i + 1;
            day = day + 1;
            break;
        }
        day -= month_lengths[i];
    }

    format!("{year}-{month:02}-{day:02}_{hour:02}:{minute:02}:{second:02}")
}
