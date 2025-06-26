use anyhow::Result;
use byteorder::{NativeEndian, ReadBytesExt};
use csv::ReaderBuilder;
use log::{info, LevelFilter, log};
use rust_embed::RustEmbed;
use clap::Parser;
use sameold::{Message, SameReceiverBuilder, SignificanceLevel};
use serde::Deserialize;
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::env::args;
use std::io::{self};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use std::process::{Command, Stdio};
use strum::EnumMessage;

#[derive(RustEmbed)]
#[folder = "src"]
struct Asset;

#[derive(Debug, Deserialize)]
struct Record {
    code: String,
    county: String,
    state: String,
}

struct MessageSender {
    last_message_time: Option<Instant>,
}

impl MessageSender {
    fn new() -> Self {
        MessageSender {
            last_message_time: None,
        }
    }

    async fn send_message_with_retry(
        &mut self,
        chan: u32,
        message: &str,
        retries: u32,
        delay: Duration,
        args: Args,
    ) -> Result<(), String> {
        // Ensure at least 20 seconds between messages
        if let Some(last_time) = self.last_message_time {
            let elapsed = last_time.elapsed();
            if elapsed < Duration::from_secs(20) {
                sleep(Duration::from_secs(20) - elapsed).await;
            }
        }

        for attempt in 0..=retries {
            // Create a new Command instance
            let mut command = Command::new("meshtastic");
                command.arg("--ch-index");
                command.arg(chan.to_string());
                command.arg("--sendtext");
                command.arg(message.to_string()); // Convert message to String to extend its lifetime
                command.arg("--ack");

            // Conditionally add the host argument if provided
            if let Some(host) = &args.host {
                command.arg("--host").arg(host);
            }
            
            // Conditionally add the port argument if provided
            if let Some(port) = &args.port {
                command.arg("--port").arg(port);
            }

            // Execute the command
            let result = command.spawn();

            match result {
                Ok(_) => {
                    self.last_message_time = Some(Instant::now());
                    return Ok(());
                }
                Err(e) => {
                    if attempt < retries {
                        log::warn!("Error sending message: {}. Retrying in {:?}...", e, delay);
                        sleep(delay).await;
                    } else {
                        log::error!("Error sending message after {} attempts: {}", retries, e);
                        return Err(format!("Failed to send message: {}", e));
                    }
                }
            }
        }
        Ok(())
    }



}

async fn check_node_connection(args: Args) -> Result<()> {
    // Construct the command to run `meshtastic --info`
    let mut cmd = Command::new("meshtastic");



    // Conditionally add the "--host" argument if the host is provided
    if let Some(host) = &args.host {
        cmd.arg("--host");
        cmd.arg(host);  // Add host argument here
    }

    // Conditionally add the "--port" argument if the serial port is provided ie. /dev/ttyUSB0
    if let Some(port) = &args.port {
        cmd.arg("--port");
        cmd.arg(port);  // Add port argument here
    }


    // Add the --info argument
    cmd.arg("--info");

    // Ensure the command doesn't output to the console
    cmd.stdout(Stdio::piped());

    // Run the command and capture the output
    let output = cmd.output();

    match output {
        Ok(output) => {
            // Convert the stdout to a string (output is captured as bytes)
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Check if the output contains "Error"
            if stdout.contains("Error") {
                log::error!("Received error output: {}", stdout);
                std::process::exit(1);
            }


            // Check the first line of the output
            if let Some(first_line) = stdout.lines().next() {
                if first_line == "Connected to radio" {
                    log::info!("Successfully connected to the node.");
                    return Ok(());
                } else {
                    log::error!("Failed to connect to the radio. First line: {}", first_line);
                    std::process::exit(1);
                }
            } else {
                log::error!("Output from meshtastic --info was empty.");
                std::process::exit(1);
            }
        }
        Err(e) => {
            // Log error if the command failed to run
            log::error!("Failed to execute meshtastic --info: {}", e);
            std::process::exit(1);
        }
    }

}

async fn load_csv_into_hashmap() -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();

    let csv_data = Asset::get("sameCodes.csv").unwrap();
    let csv_str = std::str::from_utf8(csv_data.data.as_ref()).unwrap();

    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv_str.as_bytes());

    for result in rdr.deserialize() {
        let record: Record = result.unwrap();
        map.insert(record.code, (record.county, record.state));
    }

    map
}


fn search_by_code<'a>(
    map: &'a HashMap<String, (String, String)>,
    code: &str,
) -> Option<&'a (String, String)> {
    map.get(code)
}

#[derive(Parser, Debug)]
#[command(long_about = None)]
struct Args {
    /// Channel to which alerts are sent to, if not provided will default to channel 0
    #[arg(long, short)]
    alert_channel: Option<u32>,

    /// Channel to which tests are sent to, if not provided tests will be ignored
    #[arg(long, short)]
    test_channel: Option<u32>,

    /// Network address with port of device to connect to in the form of target.address:port
    #[arg(long)]
    host: Option<String>,

    /// Sample rate.
    #[arg(long, short, default_value_t = 48000)]
    rate: u32,

    /// Location codes that must be present to send an alert
    #[arg(long, short, value_delimiter = ',', default_value = None, required = false)]
    locations: Vec<String>,

}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("Meshtastic_SAME_EAS_Alerter", LevelFilter::Info)
        .init()
        .unwrap();

    // Default channel for alerts
    let mut alert_channel: u32 = 0;

    // Default value of test_channel
    // 10 means that tests will not be logged
    let mut test_channel: u32 = 10;

    // Parse the command line arguments
    let args = Args::parse();


    // Handle alertChannel argument
    if let Some(alert_channel_arg) = args.alert_channel {
        if !(0..=7).contains(&alert_channel_arg) {
            // https://meshtastic.org/docs/configuration/radio/channels/
            return Err(anyhow::anyhow!("alertChannel must be between 0 and 7"));
        } else {
            alert_channel = alert_channel_arg;
        }
    }

    // Handle testChannel argument
    if let Some(test_channel_arg) = args.test_channel {
        if !(0..=7).contains(&test_channel_arg) {
            // https://meshtastic.org/docs/configuration/radio/channels/
            return Err(anyhow::anyhow!("testChannel must be between 0 and 7"));
        } else {
            test_channel = test_channel_arg;
        }
    }

    check_node_connection(Args::parse()).await.expect("Failed to check node connection");

    // Create a SameReceiver.
    let mut rx = SameReceiverBuilder::new(args.rate)
        .with_agc_gain_limits(1.0f32 / (i16::MAX as f32), 1.0 / 200.0)
        .with_agc_bandwidth(0.05) // AGC bandwidth at symbol rate, < 1.0
        .with_squelch_power(0.10, 0.05) // squelch open/close power, 0.0 < power < 1.0
        .with_preamble_max_errors(2) // bit error limit when detecting sync sequence
        .build();

    // Set up stdin as the input source
    let stdin = io::stdin();
    // Check if there is any input from stdin
    if atty::is(atty::Stream::Stdin) {
        log::error!("Error: No input provided to stdin. Please provide RTL FM input.");
        std::process::exit(1);
    }

    let map = load_csv_into_hashmap().await;
    log::info!("Loaded locations CSV");

    let stdin_handle = stdin.lock();
    let mut inbuf = Box::new(io::BufReader::new(stdin_handle));

    // Create an iterator for audio source from stdin, reading i16 and converting to f32
    let audiosrc = std::iter::from_fn(|| inbuf.read_i16::<NativeEndian>().ok());

    let mut sender = MessageSender::new();

    log::info!("Monitoring for alerts");
    log::info!("Alerts will be sent to channel: {}", alert_channel);
    if test_channel == 10 {
        log::info!("Tests alerts will be ignored (test-channel argument was not provided)")
    } else {
        log::info!("Test alerts will be sent to channel: {}", test_channel)
    }

    // Process messages from the audio source
    for msg in rx.iter_messages(audiosrc.map(|sa| sa as f32)) {
        match msg {
            Message::StartOfMessage(hdr) => {
                let evt = hdr.event();
                log::info!("Begin SAME voice message: {:?}", hdr);
                let mut message: String;
                let mut send_channel: u32 = alert_channel;

                message = ", Issued By: ".to_string()
                    + hdr.originator().get_detailed_message().unwrap();
                match evt.significance() {
                    SignificanceLevel::Test => {
                        send_channel = test_channel;
                        if test_channel == 10 {
                            log::info!("Ignoring test alert");
                            continue;
                        }
                        message = "📖Received ".to_string()
                            + &evt.to_string()
                            + " from "
                            + hdr.callsign()
                            + &*message;
                    }
                    SignificanceLevel::Statement => {
                        message = "📟".to_string() + &evt.to_string() + &*message;
                    }
                    SignificanceLevel::Emergency => {
                        message = "🚨".to_string() + &evt.to_string() + &*message;
                    }
                    SignificanceLevel::Watch => {
                        message = "⚠️".to_string() + &evt.to_string() + &*message;
                    }
                    SignificanceLevel::Warning => {
                        message = "🚨".to_string() + &evt.to_string() + &*message;
                    }
                    SignificanceLevel::Unknown => {
                        message = "🚨".to_string() + &evt.to_string() + &*message;
                    }
                }
                let codes: Vec<String> = hdr.location_str_iter().map(|s| s.to_string()).collect();

                if hdr.is_national() {
                    message += " Nationwide Alert"
                } else {
                    if !args.locations.is_empty() && !codes.is_empty() {
                        // Log the values for debugging
                        log::debug!("Provided locations: {:?}", args.locations);
                        log::debug!("Alert locations: {:?}", codes);

                        let has_match = codes.iter().any(|code| {
                            let matches = args.locations.contains(code);
                            log::debug!("Comparing alert code '{}' with provided locations: {}", code, matches);
                            matches
                        });

                        if !has_match {
                            log::info!("Ignoring alert with no matching locations in filter");
                            continue;
                        } else {
                            log::info!("Alert has matching locations, proceeding to send");
                        }
                    } else {
                        log::info!("No location filter applied (locations empty) or no locations in alert");
                    }

                    let mut locations_found = Vec::new();

                    // Pass each code into the function and collect the results
                    for code in codes {
                        if let Some((county, _state)) =
                            search_by_code(&map, &format!("0{}", &code[1..]))
                        {
                            let mut location = String::new();

                            // Determining where in the county the location is
                            // https://www.weather.gov/nwr/sameenz
                            match code.chars().next().unwrap_or_default() {
                                '0' => {}
                                '1' => location.push_str("Northwest "),
                                '2' => location.push_str("North "),
                                '3' => location.push_str("Northeast "),
                                '4' => location.push_str("West "),
                                '5' => location.push_str("Central "),
                                '6' => location.push_str("East "),
                                '7' => location.push_str("Southwest "),
                                '8' => location.push_str("South "),
                                '9' => location.push_str("Southeast "),
                                _ => {}
                            }

                            location.push_str(county);
                            locations_found.push(location);
                        } else {
                            log::debug!("Location Code: {} not found", code);
                        }
                    }

                    if !locations_found.is_empty() {
                        if locations_found.len() == 1 {
                            message.push_str(", Location: ");
                        } else {
                            message.push_str(", Locations: ");
                        }
                        message.push_str(&locations_found.join(", "));
                    }
                }

                log::info!("Attempting to send message over the mesh: {}", message);

                // Split and send the message in chunks of 75 characters, using retry logic
                let mut myvec: Vec<usize> = message.bytes().enumerate().filter(|(_,c)| *c == b' ').map(|(i,_)| i).collect::<Vec<_>>();
                let mut curpos: usize = 0;
                let mut curlen: usize = 0;
                let mut startpos: usize = 0;
                for i in myvec.iter_mut() {
                    if curlen + *i - curpos > 75 {
                        sender
                            .send_message_with_retry(send_channel, &message[startpos..(startpos + curlen)], 3, Duration::from_secs(5), Args::parse())
                            .await.expect("Failed sending msg");
                        curpos = startpos + curlen;
                        startpos += curlen;
                        curlen = 0;
                    } else {
                        curlen += *i - curpos;
                        curpos = *i;
                    }
                }
                curlen = message.len() - startpos;
                if curlen != 0 {
                    sender
                        .send_message_with_retry(send_channel, &message[startpos..(startpos + curlen)], 3, Duration::from_secs(5), Args::parse())
                        .await.expect("Failed sending msg");
                }
            }
            Message::EndOfMessage => {
                log::info!("End SAME voice message");
            }
        }
    }

    log::warn!("Program stopped, no longer monitoring");

    Ok(())
}
