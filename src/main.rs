use anyhow::Result;
use byteorder::{NativeEndian, ReadBytesExt};
use clap::{ArgGroup, Parser};
use csv::ReaderBuilder;
use log::LevelFilter;
use meshtastic::api::StreamApi;
use meshtastic::packet::PacketDestination::Broadcast;
use meshtastic::packet::PacketRouter;
use meshtastic::protobufs::{FromRadio, MeshPacket};
use meshtastic::types::{MeshChannel, NodeId};
use meshtastic::utils;
use rust_embed::RustEmbed;
use sameold::{Message, SameReceiverBuilder, SignificanceLevel};
use serde::Deserialize;
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::io::{self};
use strum::{Display, EnumMessage};
use thiserror::Error;

#[derive(RustEmbed)]
#[folder = "src"]
struct Asset;

#[derive(Debug, Deserialize)]
struct Record {
    code: String,
    county: String,
    state: String,
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
#[command(group(ArgGroup::new("operation").required(true).args(&["port", "ports", "host"])))]
struct Args {
    /// Serial port of device to connect to
    #[arg(long, short)]
    port: Option<String>,

    /// Network address with port of device to connect to
    #[arg(long)]
    host: Option<String>,

    /// Flag to print all open ports, use it to find the correct port
    #[arg(long)]
    ports: bool,

    /// Channel to which alerts are sent to, if not provided will default to channel 0
    #[structopt(long, short)]
    alert_channel: Option<u32>,

    /// Channel to which tests are sent to, if not provided tests will be ignored
    #[structopt(long, short)]
    test_channel: Option<u32>,

    /// Sample rate.
    #[arg(long, short, default_value_t=48000)]
    rate: u32
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

    // Check if the --ports flag is set
    if args.ports {
        let available_ports = utils::stream::available_serial_ports()?;
        println!("Available ports: {:?}", available_ports);
        return Ok(());
    }

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

    let unconnected_stream_api = StreamApi::new();
    let stream_api = if let Some(port) = args.port {
        let stream = utils::stream::build_serial_stream(port.clone(), None, None, None)?;
        let (_decoded_listener, stream_api) = unconnected_stream_api.connect(stream).await;
        log::info!("Connected to device via serial port on {:?}", port);
        stream_api
    } else if let Some(host) = args.host {
        let stream = utils::stream::build_tcp_stream(host.clone()).await?;
        let (_decoded_listener, stream_api) = unconnected_stream_api.connect(stream).await;
        log::info!("Connected to device via TCP on {:?}", host);
        stream_api
    } else {
        unreachable!();
    };

    let config_id = utils::generate_rand_id();
    let mut packet_router = MyPacketRouter::new(0);
    let mut meshtastic_stream = stream_api.configure(config_id).await?;

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
                let mut channel: MeshChannel = alert_channel.into();

                message = ", Issued By: ".to_string()
                    + hdr.originator().get_detailed_message().unwrap();
                match evt.significance() {
                    SignificanceLevel::Test => {
                        if test_channel == 10 {
                            log::info!("Ignoring test alert");
                            continue;
                        }
                        message = "📖Received ".to_string()
                            + &evt.to_string()
                            + " from "
                            + hdr.callsign()
                            + &*message;
                        channel = test_channel.into();
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
                let codes: Vec<String> =
                    hdr.location_str_iter().map(|s| s.to_string()).collect();
                if hdr.is_national() {
                    message += " Nationwide Alert"
                } else {
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

                if message.len() > 228 {
                    log::debug!("Message string too long for Meshtastic, truncating");
                    message.truncate(228);
                }

                log::info!("Attempting to send message over the mesh: {}", message);

                // Attempt to send message over the mesh
                if let Err(e) = meshtastic_stream
                    .send_text(&mut packet_router, message, Broadcast, true, channel)
                    .await
                {
                    log::error!("Error sending message: {}", e);
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

#[allow(unused)]
#[derive(Display, Clone, Error, Debug)]
pub enum DeviceUpdateError {
    PacketNotSupported(String),
    RadioMessageNotSupported(String),
    DecodeFailure(String),
    GeneralFailure(String),
    EventDispatchFailure(String),
    NotificationDispatchFailure(String),
}
#[allow(unused)]
#[derive(Default)]
struct MyPacketRouter {
    _source_node_id: NodeId,
}

impl MyPacketRouter {
    fn new(node_id: u32) -> Self {
        MyPacketRouter {
            _source_node_id: node_id.into(),
        }
    }
}
#[allow(unused)]
impl PacketRouter<(), DeviceUpdateError> for MyPacketRouter {
    fn handle_packet_from_radio(
        &mut self,
        _packet: FromRadio,
    ) -> std::result::Result<(), DeviceUpdateError> {
        Ok(())
    }

    fn handle_mesh_packet(
        &mut self,
        _packet: MeshPacket,
    ) -> std::result::Result<(), DeviceUpdateError> {
        Ok(())
    }

    fn source_node_id(&self) -> NodeId {
        self._source_node_id
    }
}
