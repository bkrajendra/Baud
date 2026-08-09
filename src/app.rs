// src/app.rs
use crate::format::{format_timestamp, LineEnding};
use crate::linebuf::LineAssembler;
use crate::serial::{list_ports, SerialConnection, SerialEvent};

const BAUD_RATES: [u32; 6] = [9600, 19200, 38400, 57600, 115200, 230400];

pub struct BaudApp {
    available_ports: Vec<String>,
    selected_port: Option<String>,
    baud_rate: u32,
    custom_baud_selected: bool,
    custom_baud_text: String,

    connection: Option<SerialConnection>,
    line_assembler: LineAssembler,

    lines: Vec<(String, String)>, // (timestamp, text)
    show_timestamps: bool,
    autoscroll: bool,

    input_text: String,
    line_ending: LineEnding,

    dark_mode: bool,
    error_message: Option<String>,
}

impl Default for BaudApp {
    fn default() -> Self {
        Self {
            available_ports: list_ports(),
            selected_port: None,
            baud_rate: 115200,
            custom_baud_selected: false,
            custom_baud_text: String::new(),
            connection: None,
            line_assembler: LineAssembler::new(),
            lines: Vec::new(),
            show_timestamps: false,
            autoscroll: true,
            input_text: String::new(),
            line_ending: LineEnding::Lf,
            dark_mode: true,
            error_message: None,
        }
    }
}

impl BaudApp {
    fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    fn connect(&mut self) {
        let Some(port_name) = self.selected_port.clone() else {
            self.error_message = Some("Select a port first".to_string());
            return;
        };
        match SerialConnection::open(&port_name, self.baud_rate) {
            Ok(conn) => {
                self.connection = Some(conn);
                self.error_message = None;
            }
            Err(e) => self.error_message = Some(e),
        }
    }

    fn disconnect(&mut self) {
        if let Some(conn) = self.connection.take() {
            conn.close();
        }
        if let Some(line) = self.line_assembler.flush() {
            self.push_line(line);
        }
    }

    fn push_line(&mut self, text: String) {
        let ts = format_timestamp(chrono::Local::now());
        self.lines.push((ts, text));
    }

    fn drain_serial_events(&mut self) {
        let mut events = Vec::new();
        if let Some(conn) = &self.connection {
            while let Ok(event) = conn.events().try_recv() {
                events.push(event);
            }
        }

        let mut disconnected = false;
        for event in events {
            match event {
                SerialEvent::Data(bytes) => {
                    for line in self.line_assembler.push(&bytes) {
                        self.push_line(line);
                    }
                }
                SerialEvent::Error(e) => {
                    self.error_message = Some(e);
                }
                SerialEvent::Closed => {
                    disconnected = true;
                }
            }
        }
        if disconnected {
            if let Some(conn) = self.connection.take() {
                conn.close();
            }
            if let Some(line) = self.line_assembler.flush() {
                self.push_line(line);
            }
        }
    }
}

impl eframe::App for BaudApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;

        self.drain_serial_events();
        if self.is_connected() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        ctx.set_visuals(if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });

        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!self.is_connected(), |ui| {
                    egui::ComboBox::from_id_salt("port_combo")
                        .selected_text(self.selected_port.clone().unwrap_or_else(|| "Select port".to_string()))
                        .show_ui(ui, |ui| {
                            for port in &self.available_ports {
                                ui.selectable_value(&mut self.selected_port, Some(port.clone()), port);
                            }
                        });

                    if ui.button("Refresh").clicked() {
                        self.available_ports = list_ports();
                    }

                    egui::ComboBox::from_id_salt("baud_combo")
                        .selected_text(if self.custom_baud_selected {
                            "Custom".to_string()
                        } else {
                            self.baud_rate.to_string()
                        })
                        .show_ui(ui, |ui| {
                            for rate in BAUD_RATES {
                                let selected = !self.custom_baud_selected && self.baud_rate == rate;
                                if ui.selectable_label(selected, rate.to_string()).clicked() {
                                    self.baud_rate = rate;
                                    self.custom_baud_selected = false;
                                }
                            }
                            if ui.selectable_label(self.custom_baud_selected, "Custom").clicked() {
                                self.custom_baud_selected = true;
                            }
                        });

                    if self.custom_baud_selected {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.custom_baud_text)
                                .hint_text("custom")
                                .desired_width(60.0),
                        );
                        if ui.button("Use").clicked() {
                            if let Ok(rate) = self.custom_baud_text.trim().parse::<u32>() {
                                self.baud_rate = rate;
                            } else {
                                self.error_message = Some("Custom baud must be a number".to_string());
                            }
                        }
                    }
                });

                let connect_label = if self.is_connected() { "Disconnect" } else { "Connect" };
                if ui.button(connect_label).clicked() {
                    if self.is_connected() {
                        self.disconnect();
                    } else {
                        self.connect();
                    }
                }

                ui.checkbox(&mut self.show_timestamps, "Timestamps");
                ui.checkbox(&mut self.dark_mode, "Dark mode");
            });

            if let Some(err) = self.error_message.clone() {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::RED, &err);
                    if ui.button("Dismiss").clicked() {
                        self.error_message = None;
                    }
                });
            }
        });

        egui::Panel::bottom("input_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                let response = ui.add_enabled(
                    self.is_connected(),
                    egui::TextEdit::singleline(&mut self.input_text).desired_width(400.0),
                );

                egui::ComboBox::from_id_salt("line_ending_combo")
                    .selected_text(self.line_ending.to_string())
                    .show_ui(ui, |ui| {
                        for le in LineEnding::ALL {
                            ui.selectable_value(&mut self.line_ending, le, le.to_string());
                        }
                    });

                let send_clicked = ui
                    .add_enabled(self.is_connected(), egui::Button::new("Send"))
                    .clicked();
                let enter_pressed = response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));

                if self.is_connected() && (send_clicked || enter_pressed) {
                    let mut bytes = self.input_text.clone().into_bytes();
                    bytes.extend_from_slice(self.line_ending.as_bytes());
                    if let Some(conn) = &mut self.connection {
                        if let Err(e) = conn.send(&bytes) {
                            self.error_message = Some(e);
                        } else {
                            self.input_text.clear();
                        }
                    }
                    response.request_focus();
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.autoscroll, "Autoscroll");
                if ui.button("Clear").clicked() {
                    self.lines.clear();
                }
            });
            ui.separator();

            let mut scroll_area = egui::ScrollArea::vertical();
            if self.autoscroll {
                scroll_area = scroll_area.stick_to_bottom(true);
            }
            scroll_area.show(ui, |ui| {
                for (ts, text) in &self.lines {
                    if self.show_timestamps {
                        ui.label(format!("[{ts}] {text}"));
                    } else {
                        ui.label(text);
                    }
                }
            });
        });
    }
}
