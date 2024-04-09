mod api;
mod serial;
mod server;
use api::SendToGui;

// Logging
use log::{debug, error, info};

// Network
use message_io::network::{Endpoint, SendStatus, Transport};
use message_io::node::{self, NodeHandler};
use std::net::ToSocketAddrs;

// Threads
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

// Arguments
use clap::Parser;

// Other
use crate::api::ThreadCom;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    #[arg(short, long, help = "Network port to use", default_value_t = 24377)]
    port: u16,
}

pub fn send_network(
    network_handler: &NodeHandler<()>,
    endpoint: Option<Endpoint>,
    message: SendToGui,
) {
    if let Some(endpoint) = endpoint {
        let output_data = bincode::serialize(&message).unwrap();
        let status = network_handler.network().send(endpoint, &output_data);
        //debug!("Status of message {:?} is {:?}", message, status);
        if status != SendStatus::Sent {
            error!("Packet not send?");
        }
    } else {
        error!("Failed to send network message: missing endpoint");
    }
}

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default()
            .filter_or(env_logger::DEFAULT_FILTER_ENV, "none,ws-serial-tcp=debug"),
    );
    debug!("Starting ws-serial-tcp");

    let args = Args::parse();

    let mut endpoint_saved: Option<Endpoint> = None;

    // Threads
    let (mut tx_gui, rx_gui) = mpsc::channel();
    let (tx_main, rx_main) = mpsc::channel();
    let (tx_serial, mut rx_serial) = mpsc::channel();

    // Network
    let addr = ("0.0.0.0", args.port)
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    let (handler, listener) = node::split::<()>();
    let network_handler = Arc::new(handler);
    let transport = Transport::Ws;
    match network_handler.network().listen(transport, addr) {
        Ok((_id, real_addr)) => info!("Server running at {} by {}", real_addr, transport),
        Err(_) => error!("Can not listening at {} by {}", addr, transport),
    }

    let network_handler_server = network_handler.clone();
    thread::spawn(move || {
        server::run(network_handler_server, listener, tx_serial, tx_main);
    });

    // This will wait untill connection
    if let Ok(event) = rx_main.recv() {
        match event {
            ThreadCom::ClientConnected(endpoint, _resource_id) => {
                info!("Server received: ClientConnected");
                endpoint_saved = Some(endpoint);
            }
        }
    }

    thread::spawn(move || {
        serial::main(&mut tx_gui, &mut rx_serial);
    });

    loop {
        match rx_gui.recv() {
            Ok(x) => {
                send_network(&network_handler.clone(), endpoint_saved, x);
            }
            Err(x) => {
                error!("Failed to recv {}", x);
            }
        }
    }
}
