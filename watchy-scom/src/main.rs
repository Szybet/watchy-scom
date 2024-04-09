#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

pub mod defines;
pub mod serial;

use crate::SendToSerial::*;
use defines::{SendToGui, SendToSerial};
use eframe::egui;
use egui::{Color32, Vec2};
use egui_extras::RetainedImage;
use log::{debug, error};
use regex::Regex;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), eframe::Error> {
    let (tx_serial, mut rx_serial) = channel();
    let (mut tx_gui, rx_gui) = channel();

    thread::spawn(move || {
        serial::main(&mut tx_gui, &mut rx_serial);
    });

    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 650.0]),
        ..Default::default()
    };

    eframe::run_native(
        "watchy-scom",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::new(MyApp::new(tx_serial, rx_gui))
        }),
    )
}

struct MyApp {
    tx_serial: Sender<SendToSerial>,
    rx_gui: Receiver<SendToGui>,
    sel_port: usize,
    ports: Vec<String>,
    baud_rate: String,
    image: Vec<u8>,
    logs: String,
    connected: bool,
}

impl MyApp {
    pub fn new(tx_serial: Sender<SendToSerial>, rx_gui: Receiver<SendToGui>) -> Self {
        Self {
            tx_serial,
            rx_gui,
            sel_port: 0,
            ports: Vec::new(),
            baud_rate: String::from("921600"),
            image: Vec::new(),
            logs: String::new(),
            connected: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.rx_gui.recv_timeout(Duration::from_millis(25)) {
                Ok(x) => match x {
                    SendToGui::Ports(x) => {
                        self.ports = x;
                    }
                    SendToGui::LogToShow(input) => {
                        debug!("Received logs to show: {}", input);
                        // Cleans
                        let re = Regex::new(r"^src/.*").unwrap();
                        let mut filtered_lines = String::new();
                        for line in input.lines() {
                            if re.is_match(line) {
                                filtered_lines.push_str(line);
                                filtered_lines.push('\n'); // Add a newline character to separate lines
                            } else {
                                if !line.is_empty() {
                                    error!("Rejected line: {}", line);
                                    let regex = Regex::new("[^\x00-\x7F]").unwrap();
                                
                                    let matches: Vec<_> = regex.find_iter(line).collect();
                                
                                    // Check if the number of weird bytes exceeds 30
                                    if matches.len() > 30 {
                                        debug!("We probably catched the scren, requesting an update...");
                                        self.tx_serial
                                            .send(SendMessage("screen:".to_string()))
                                            .unwrap();
                                    }
                                }
                            }
                        }

                        filtered_lines = filtered_lines.replace("\n\n", "\n");
                        //let result2 = re_non_standard.replace_all(&result, "");
                        self.logs.push_str(&filtered_lines.clone()); // not sure about the clone
                    }
                    SendToGui::ShowPng(x) => {
                        debug!("Received png");
                        self.image = x;
                    }
                },
                Err(_x) => {
                    /*
                    if _x.to_string() != "receiving on an empty channel" {
                        error!("Failed to recv in gui: {}", _x);
                    }
                    */
                }
            }

            egui::CentralPanel::default().show_inside(ui, |ui| {
                egui::TopBottomPanel::top("top_panel")
                    .resizable(false)
                    .min_height(130.0)
                    .show_inside(ui, |ui| {
                        ui.heading("Settings");

                        ui.label("Input the baud rate");

                        ui.add(egui::TextEdit::singleline(&mut self.baud_rate));

                        if ui.add(egui::Button::new("Scan for ports")).clicked() {
                            if self.tx_serial.send(AskForPorts()).is_err() {
                                error!("Failed to ask for ports");
                            }
                        }

                        if !self.ports.is_empty() {
                            egui::ComboBox::from_label("Select the port").show_index(
                                ui,
                                &mut self.sel_port,
                                self.ports.len(),
                                |i| self.ports[i].clone(),
                            );
                        }

                        if self.sel_port != 0 {
                            if ui
                                .add(egui::Button::new(format!(
                                    "Connect to {} with baud rate {}",
                                    self.ports[self.sel_port].clone(),
                                    self.baud_rate
                                )))
                                .clicked()
                            {
                                let baud_rate: usize = self.baud_rate.parse().unwrap();
                                if self
                                    .tx_serial
                                    .send(SelectPort(self.ports[self.sel_port].clone(), baud_rate))
                                    .is_err()
                                {
                                    error!("Failed to ask for ports");
                                }
                                self.connected = true;
                            }
                        }
                    });
                if self.connected {
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Back")).clicked() {
                            debug!("Button to button clicked");
                            self.tx_serial
                                .send(SendMessage("back-button:".to_string()))
                                .unwrap();
                        }
                        if ui.add(egui::Button::new("Menu")).clicked() {
                            debug!("Button to button clicked");
                            self.tx_serial
                                .send(SendMessage("menu-button:".to_string()))
                                .unwrap();
                        }
                        if ui.add(egui::Button::new("Up")).clicked() {
                            debug!("Button to button clicked");
                            self.tx_serial
                                .send(SendMessage("up-button:".to_string()))
                                .unwrap();
                        }
                        if ui.add(egui::Button::new("Down")).clicked() {
                            debug!("Button to button clicked");
                            self.tx_serial
                                .send(SendMessage("down-button:".to_string()))
                                .unwrap();
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Long back")).clicked() {
                            debug!("Button to button clicked");
                            self.tx_serial
                                .send(SendMessage("long-back-button:".to_string()))
                                .unwrap();
                        }
                        if ui.add(egui::Button::new("Long menu")).clicked() {
                            debug!("Button to button clicked");
                            self.tx_serial
                                .send(SendMessage("long-menu-button:".to_string()))
                                .unwrap();
                        }
                        if ui.add(egui::Button::new("Long up")).clicked() {
                            debug!("Button to button clicked");
                            self.tx_serial
                                .send(SendMessage("long-up-button:".to_string()))
                                .unwrap();
                        }
                        if ui.add(egui::Button::new("Long down")).clicked() {
                            debug!("Button to button clicked");
                            self.tx_serial
                                .send(SendMessage("long-down-button:".to_string()))
                                .unwrap();
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Update screen")).clicked() {
                            debug!("Button to update screen clicked");
                            self.tx_serial
                                .send(SendMessage("screen:".to_string()))
                                .unwrap();
                        }
                        if !self.image.is_empty() {
                            if ui.add(egui::Button::new("Open screen")).clicked() {
                                debug!("Button to save image clicked");
                                let _ = std::fs::write("/tmp/watchy-scom.png", &self.image.clone());
                                Command::new("kolourpaint")
                                    .arg("/tmp/watchy-scom.png")
                                    .spawn()
                                    .expect("failed to execute process");
                            }
                            if ui.add(egui::Button::new("Reset")).clicked() {
                                debug!("Button to reset the watchy clicked");
                                self.tx_serial
                                    .send(SendMessage("reset:".to_string()))
                                    .unwrap();
                            }
                        }
                    });
                }
                if !self.image.is_empty() {
                    ui.horizontal_centered(|ui| {
                        //debug!("Showing image");
                        //let _ = std::fs::write("output.png", &self.image.clone());
                        RetainedImage::from_image_bytes("png", &self.image)
                            .unwrap()
                            .show_size(ui, Vec2::new(400.0, 400.0));
                        //let eimg = egui::Image::from_bytes("", self.image.clone()).fit_to_exact_size(Vec2::new(200.0, 200.0)).show_loading_spinner(true);
                        //let uh = ui.add(eimg);
                    });
                }
            });

            egui::SidePanel::right("right_panel")
                .resizable(true)
                .default_width(600.0)
                .show_inside(ui, |ui| {
                    //debug!("self.logs: {}", self.logs);
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add_sized(
                                ui.available_size(),
                                egui::TextEdit::multiline(&mut self.logs)
                                    .text_color(Color32::WHITE)
                                    .interactive(true),
                            );
                        });
                });

            ctx.request_repaint_after(Duration::from_millis(40));
        });
    }
}
