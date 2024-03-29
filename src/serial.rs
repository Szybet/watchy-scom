use crate::defines::SendToSerial::*;
use crate::SendToGui::*;
use image::{ImageBuffer, ImageFormat, Rgb};
use log::{debug, error, info};
use serialport::{self, SerialPort};
use std::{
    io::Read,
    process::exit,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

pub fn main(
    tx_gui: &mut Sender<crate::defines::SendToGui>,
    rx_serial: &mut Receiver<crate::defines::SendToSerial>,
) {
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut serial_buf: Vec<u8> = Vec::with_capacity(16000); // 15000 is screen size
    let mut synced = false;
    loop {
        match rx_serial.recv_timeout(Duration::from_millis(110)) {
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
                            .timeout(Duration::from_millis(30000))
                            .open()
                            .expect("Failed to open port"),
                    );
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
            Err(x) => {
                /*
                if x.to_string() != "receiving on an empty channel" {
                    error!("Failed to recv in serial: {}", x);
                }
                */
            }
        }
        if let Some(ref mut rport) = port {
            //debug!("Reading from port...");
            let mut serial_buf_tmp: Vec<u8> = vec![0; 7000];
            let readed = rport.read(serial_buf_tmp.as_mut_slice()).unwrap();
            //debug!("Readed bytes: {}", readed);
            //debug!("Pure dump: {}", String::from_utf8_lossy(&serial_buf_tmp));
            let mut done = false;
            // eof
            if serial_buf_tmp.contains(&26) {
                //debug!("it contains eof!");
                if !synced {
                    synced = true;
                    serial_buf.clear();
                    serial_buf_tmp.clear();
                    debug!("SYNCED!");
                    continue;
                }
                done = true;
            }
            serial_buf.extend(serial_buf_tmp);
            if done {
                //debug!("serial_buf len: {}", serial_buf.len());
                let mut logs: Vec<u8> = Vec::new();
                let mut screen: Vec<u8> = Vec::new();
                let mut rest: Vec<u8> = Vec::new();
                let mut screen_now = false;
                let mut screen_done = false;
                for b in serial_buf.clone() {
                    if !screen_now {
                        if b == 140 {
                            screen_now = true;
                        } else {
                            logs.push(b);
                        }
                    } else {
                        if b == 26 {
                            screen_done = true;
                        } else {
                            if !screen_done {
                                screen.push(b);
                            } else {
                                rest.push(b);
                            }
                        }
                    }
                }
                serial_buf.clear();
                serial_buf.extend(rest);

                let real_logs = String::from_utf8_lossy(&logs);
                //debug!("Real logs: {}", real_logs);
                if tx_gui.send(LogToShow(real_logs.to_string())).is_err() {
                    error!("Failed to send logs to gui");
                }

                // No screen in there
                if !screen_now {
                    //debug!("There is no screen");
                    continue;
                }

                let screen_stringed: Vec<char> = String::from_utf8_lossy(&screen)
                    .chars()
                    .filter(|c| c.is_whitespace() || c.is_ascii_graphic())
                    .collect(); // thats the solution to the len problem
                                // Fixed value of how many there should be
                if screen_stringed.len() != 15000 {
                    debug!("Screen len is: {}", screen.len());
                    error!("screen_stringed len: {}", screen_stringed.len());
                    continue;
                }
                let mut real_screen: Vec<u8> = Vec::new();
                let mut success = true;
                for i in (0..screen_stringed.len() - 1).step_by(3) {
                    let str = format!(
                        "{}{}{}",
                        screen_stringed[i],
                        screen_stringed[i + 1],
                        screen_stringed[i + 2]
                    );
                    if let Ok(real_byte) = str.parse::<u8>() {
                        real_screen.push(real_byte);
                    } else {
                        error!("The invalid byte is: {} at position: {}", str, i);
                        debug!(
                            "Screen from serial dump: '{}'",
                            String::from_utf8_lossy(&screen)
                        );
                        success = false;
                        break;
                    }
                    //let the_byte = format!("{}{}{}", char::from(screen[i]), char::from(screen[i + 1]), char::from(screen[i + 2])).parse::<u8>().unwrap();
                }
                if success {
                    info!("Screen succesfully readed");
                    debug!("real_screen len is: {}", real_screen.len());

                    info!("Creating the image");
                    let mut img = ImageBuffer::<Rgb<u8>, _>::new(200, 200);
                    for y in 0..200 {
                        for x in 0..200 {
                            let index = (y * 200 + x) / 8;
                            let bit_offset = 7 - ((y * 200 + x) % 8);

                            let bit = (real_screen[index] >> bit_offset) & 1;

                            let color = if bit == 0 {
                                Rgb([0, 0, 0])
                            } else {
                                Rgb([255, 255, 255])
                            };

                            img.put_pixel(x as u32, y as u32, color);
                        }
                    }
                    /*
                    let bytes: Vec<Result<u8, _>> = img.bytes().collect();
                    let bytes_pure: Vec<u8> = img.bytes().map(|result| result.unwrap()).collect();
                    let _ = tx_gui.send(ShowPng(bytes_pure));
                    */
                    let mut buffer = std::io::Cursor::new(Vec::new());
                    img.write_to(&mut buffer, ImageFormat::Png).unwrap();
                    if tx_gui.send(ShowPng(buffer.into_inner())).is_err() {
                        error!("Failed to send png to gui");
                    }
                }
            }
        }
    }
}
