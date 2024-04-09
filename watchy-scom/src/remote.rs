#![deny(clippy::useless_attribute)]
#![allow(clippy::single_match)]

// Logging
use log::{debug, error, info};

// Network
use message_io::network::{Endpoint, NetEvent, RemoteAddr, Transport};
use message_io::node::{self, NodeEvent, NodeHandler};

use std::sync::mpsc::{Receiver, Sender};
// Other
use std::sync::Arc;
use std::thread;

use crate::api::{SendToGui, SendToSerial};

pub fn send_data(server_id: Endpoint, handler: Arc<NodeHandler<SendToSerial>>, message: SendToSerial) {
    let output_data = bincode::serialize(&message).unwrap();
    handler.network().send(server_id, &output_data);
}

pub fn run_remote(
    remote_addr: RemoteAddr,
    tx_gui: Sender<crate::api::SendToGui>,
    rx_serial: Receiver<crate::api::SendToSerial>,
) {
    let (handler_regular, listener) = node::split();
    let handler = Arc::new(handler_regular);

    let (server_id, local_addr) = handler
        .network()
        .connect(Transport::Ws, remote_addr.clone())
        .unwrap();

    thread::spawn(move || loop {
        if let Ok(event) = rx_serial.recv() {
            send_data(server_id, handler.clone(), event);
        }
    });

    listener.for_each(move |event| match event {
        NodeEvent::Network(net_event) => match net_event {
            NetEvent::Connected(_, established) => {
                if established {
                    info!(
                        "Connected to server at {} by {}",
                        server_id.addr(),
                        Transport::Ws
                    );
                    info!("Client identified by local port: {}", local_addr.port());
                } else {
                    error!(
                        "Cannot connect to server at {} by {}",
                        remote_addr,
                        Transport::Ws
                    );
                }
            }
            NetEvent::Accepted(_, _) => unreachable!(), // Only generated when a listener accepts
            NetEvent::Message(_, input_data) => {
                debug!("Received raw input data with length: {}", input_data.len());
                let message: SendToGui = bincode::deserialize(input_data).unwrap();
                if tx_gui.send(message).is_err() {
                    error!("Failed to send message to gui");
                }
            }
            NetEvent::Disconnected(_) => {
                error!("Server is disconnected");
            }
        },
        NodeEvent::Signal(signal) => match signal {
            _ => {}
        },
    });
}
