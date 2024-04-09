use message_io::network::{Endpoint, ResourceId};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]

pub enum SendToSerial {
    AskForPorts(),
    SelectPort(String, usize),
    SendMessage(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SendToGui {
    Ports(Vec<String>),
    LogToShow(String),
    ShowPng(Vec<u8>),
}

pub enum ThreadCom {
    ClientConnected(Endpoint, ResourceId),
}