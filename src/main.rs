use std::env;
use sameold::{Message, SameReceiverBuilder};
use std::io::{self};
use std::process::{exit};
use anyhow::{Result};
use byteorder::{ReadBytesExt, NativeEndian};
use meshtastic::packet::PacketRouter;
use meshtastic::protobufs::{FromRadio, MeshPacket};
use meshtastic::types::NodeId;
use log::debug;
use meshtastic::api::StreamApi;
use meshtastic::utils;
use strum::Display;
use thiserror::Error;


#[allow(unused)]
#[tokio::main]
async fn main() -> Result<()> {

    let args: Vec<String> = env::args().collect();

    let argument = &args[1];

    //Ports Checker
    if argument.eq_ignore_ascii_case("ports"){
        let available_ports = utils::stream::available_serial_ports()?;
        println!("Available ports: {:?}", available_ports);
        exit(0);
    }

    let stream_api = StreamApi::new();

    let entered_port = argument;
    let serial_stream = utils::stream::build_serial_stream(entered_port.clone(), None, None, None)?;
    let (mut decoded_listener, stream_api) = stream_api.connect(serial_stream).await;
    println!("Connected to port: {}", entered_port);


    let config_id = utils::generate_rand_id();
    let mut meshtastic_stream = stream_api.configure(config_id).await?;

    let mut packet_router = MyPacketRouter::new(0);

    // Create a SameReceiver with a 4800 Hz audio sampling rate
    let mut rx = SameReceiverBuilder::new(48000)
        .with_agc_gain_limits(1.0f32 / (i16::MAX as f32), 1.0 / 200.0)
        .with_agc_bandwidth(0.05)          // AGC bandwidth at symbol rate, < 1.0
        .with_squelch_power(0.10, 0.05)    // squelch open/close power, 0.0 < power < 1.0
        .with_preamble_max_errors(2)       // bit error limit when detecting sync sequence
        .build();

    // Set up stdin as the input source
    let stdin = io::stdin();
    let stdin_handle = stdin.lock();
    let mut inbuf = Box::new(io::BufReader::new(stdin_handle));

    // Create an iterator for audio source from stdin, reading i16 and converting to f32
    let audiosrc = std::iter::from_fn(|| Some(inbuf.read_i16::<NativeEndian>().ok()?));

    println!("RUNNING");
    // Process messages from the audio source
    for msg in rx.iter_messages(audiosrc.map(|sa| sa as f32)) {
        match msg {
            Message::StartOfMessage(hdr) => {
                println!("Begin SAME voice message: {:?}", hdr);
                let evt = hdr.event();


            }
            Message::EndOfMessage => {
                println!("End SAME voice message");
            }
        }
    }

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
#[allow(unused)]

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
        debug!("handle_packet_from_radio called but not sure what to do");
        Ok(())
    }

    fn handle_mesh_packet(
        &mut self,
        _packet: MeshPacket,
    ) -> std::result::Result<(), DeviceUpdateError> {
        debug!("handle_mesh_packet called but not sure what to do here");
        Ok(())
    }

    fn source_node_id(&self) -> NodeId {
        self._source_node_id
    }
}


