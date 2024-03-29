pub enum SendToSerial {
    AskForPorts(),
    SelectPort(String, usize),
    SendMessage(String),
}

pub enum SendToGui {
    Ports(Vec<String>),
    LogToShow(String),
    ShowPng(Vec<u8>),
}