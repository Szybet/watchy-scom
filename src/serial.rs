use crate::defines::SendToSerial::*;
use crate::SendToGui::*;
use image::{ImageBuffer, ImageFormat, Rgb};
use log::{debug, error, info};
use serialport::{self, SerialPort};
use std::{
    io::Read,
    ops::Deref,
    sync::mpsc::{Receiver, Sender},
    thread,
    time::{self, Duration},
};

fn find_subsequence(vector: &[u8], subsequence: &[u8]) -> Option<usize> {

    if subsequence.len() as isize > vector.len() as isize - subsequence.len() as isize{
        return None;
    }

    for i in 0..=(vector.len() - subsequence.len()) {
        if vector[i..].starts_with(subsequence) {
            //debug!("Found subsequence of: {} in vec, it looks like: {}", String::from_utf8_lossy(subsequence), String::from_utf8_lossy(&vector[i..i + subsequence.len()]));
            //debug!("It's at position: {}", i);
            //debug!("Pure dump of this vector: {:?}", String::from_utf8_lossy(&vector));
            return Some(i);
        }
    }
    None
}

pub fn main(
    tx_gui: &mut Sender<crate::defines::SendToGui>,
    rx_serial: &mut Receiver<crate::defines::SendToSerial>,
) {
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut serial_buf: Vec<u8> = Vec::with_capacity(16000); // 15000 is screen size
    let mut synced = false;

    let packets_length = 16;
    // thisisastartpacket
    let start_packet: Vec<u8> = vec![
        116, 104, 105, 115, 105, 115, 97, 115, 116, 97, 114, 116, 112, 97, 99, 107,
    ];
    // thisisaendddpacket
    let end_packet: Vec<u8> = vec![
        116, 104, 105, 115, 105, 115, 97, 101, 110, 100, 100, 100, 112, 97, 99, 107,
    ];

    loop {
        match rx_serial.recv_timeout(Duration::from_millis(40)) {
            Ok(x) => match x {
                AskForPorts() => {
                    debug!("Received ask for ports");
                    match serialport::available_ports() {
                        Ok(x) => {
                            let mut serials = Vec::new();
                            serials.push(String::from("None"));
                            for serial in x {
                                serials.push(serial.port_name);
                            }
                            if tx_gui.send(Ports(serials)).is_err() {
                                error!("Failed to send Ports");
                            }
                        }
                        Err(x) => {
                            if tx_gui
                                .send(LogToShow(format!("{}", x.to_string())))
                                .is_err()
                            {
                                error!("Failed to send LogToShow");
                            }
                        }
                    }
                }
                SelectPort(port_name, baud_rate) => {
                    debug!("Received select port: {}", port_name);
                    port = Some(
                        serialport::new(port_name, baud_rate as u32) // ??? TODO: here 115_200 56000
                            .timeout(Duration::from_millis(50000))
                            .open()
                            .expect("Failed to open port"),
                    );
                    thread::sleep(time::Duration::from_millis(500));
                    if let Some(ref mut rport) = port {
                        if rport.write_all("screen:".as_bytes()).is_err() {
                            error!("Failed to write screen message");
                        }
                        if rport.flush().is_err() {
                            error!("Failed to flush");
                        };
                    }
                }
                SendMessage(x) => {
                    if let Some(ref mut rport) = port {
                        debug!("Writing to serial port: {}", x);
                        if rport.write_all(x.as_bytes()).is_err() {
                            error!("Failed to write message: {}", x);
                        }
                        if rport.flush().is_err() {
                            error!("Failed to flush");
                        };
                    } else {
                        error!("Failed to get rport");
                    }
                }
            },
            Err(_x) => {
                /*
                if _x.to_string() != "receiving on an empty channel" {
                    error!("Failed to recv in serial: {}", _x);
                }
                */
            }
        }
        if let Some(ref mut rport) = port {
            //debug!("Reading from port...");
            let mut serial_buf_tmp: Vec<u8> = vec![0; 7000];
            let _readed = rport.read(serial_buf_tmp.as_mut_slice()).unwrap();
            //debug!("Readed bytes: {}", _readed);
            //debug!("Pure dump: {}", String::from_utf8_lossy(&serial_buf_tmp));

            let real_serial_buf_tmp = &serial_buf_tmp[0.._readed].to_owned();
            serial_buf.extend(real_serial_buf_tmp);
            if let Some(end_pos) = find_subsequence(&serial_buf, &end_packet) {
                if !synced {
                    synced = true;
                    serial_buf.clear();
                    debug!("SYNCED!");
                    continue;
                }
                if let Some(start_pos) = find_subsequence(&serial_buf, &start_packet) {
                    debug!("it contains both packets!");
                    debug!("start_pos :{}", start_pos);
                    debug!("end_pos :{}", end_pos);
                    debug!("serial_buf.len(): {}", serial_buf.len());

                    if end_pos < start_pos {
                        error!("End pos is above start pos, how? skipping...");
                        serial_buf.clear();
                        continue;
                    }
                    //debug!("serial_buf len: {}", serial_buf.len());
                    let logs = serial_buf[0..start_pos].to_owned();
                    let screen = serial_buf[start_pos + packets_length..end_pos].to_owned();
                    let rest = serial_buf[end_pos + packets_length..serial_buf.len()].to_owned();

                    let real_logs = String::from_utf8_lossy(&logs);
                    //debug!("Real logs: {}", real_logs);
                    if tx_gui.send(LogToShow(real_logs.to_string())).is_err() {
                        error!("Failed to send logs to gui");
                    }

                    if screen.len() != 5000 {
                        error!("Screen len is: {}", screen.len());
                    }
                    info!("Screen succesfully readed");

                    //debug!("Real screen utf8: {}", String::from_utf8_lossy(&screen));
                    //debug!("Real screen bytes: {:?}", screen);

                    info!("Creating the image");
                    let mut img = ImageBuffer::<Rgb<u8>, _>::new(200, 200);
                    for y in 0..200 {
                        for x in 0..200 {
                            let index = (y * 200 + x) / 8;
                            let bit_offset = 7 - ((y * 200 + x) % 8);

                            let i_option = screen.get(index);
                            if let Some(i) = i_option {
                                let bit = (i >> bit_offset) & 1;

                                let color = if bit == 0 {
                                    Rgb([0, 0, 0])
                                } else {
                                    Rgb([255, 255, 255])
                                };

                                img.put_pixel(x as u32, y as u32, color);
                            } else {
                                //error!("Error creating image, pixels missing");
                                img.put_pixel(x as u32, y as u32, Rgb([255, 0, 0]));
                            }
                        }
                    }
                    let mut buffer = std::io::Cursor::new(Vec::new());
                    img.write_to(&mut buffer, ImageFormat::Png).unwrap();
                    if tx_gui.send(ShowPng(buffer.into_inner())).is_err() {
                        error!("Failed to send png to gui");
                    }

                    serial_buf.clear();
                    serial_buf.extend(rest);
                } else {
                    debug!("Found only end packet");
                    let logs = &serial_buf[0..end_pos];
                    let real_logs = String::from_utf8_lossy(&logs);
                    //debug!("Real logs: {}", real_logs);
                    if tx_gui.send(LogToShow(real_logs.to_string())).is_err() {
                        error!("Failed to send logs to gui");
                    }
                    serial_buf.clear();
                }
            }
        }
    }
}
