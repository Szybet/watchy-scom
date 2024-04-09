// Logging
use log::{debug, info};

// Network
use crate::api::*;
use message_io::network::NetEvent;
use message_io::node::{NodeHandler, NodeListener};

// Threads
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub fn run(_handler: Arc<NodeHandler<()>>, listener: NodeListener<()>, tx_to_serial: Sender<SendToSerial>, tx_to_main: Sender<ThreadCom>) {
    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(_, _) => (),
        NetEvent::Accepted(endpoint, _listener_id) => {
            // Only connection oriented protocols will generate this event
            info!("Client ({}) connected", endpoint.addr());
            let _ = tx_to_main.send(ThreadCom::ClientConnected(endpoint, _listener_id));
        }
        NetEvent::Message(_endpoint, input_data) => {
            debug!("Received raw input data with length: {}", input_data.len());
            let message: SendToSerial = bincode::deserialize(input_data).unwrap();
            match message {
                SendToSerial::AskForPorts() => {
                    let _ = tx_to_serial.send(SendToSerial::AskForPorts());
                },
                SendToSerial::SelectPort(name, baudrate) => {
                    let _ = tx_to_serial.send(SendToSerial::SelectPort(name, baudrate));
                },
                SendToSerial::SendMessage(x) => {
                    let _ = tx_to_serial.send(SendToSerial::SendMessage(x));
                },
            }
        }
        NetEvent::Disconnected(endpoint) => {
            info!("Client ({}) disconnected", endpoint.addr(),);
        }
    });
}