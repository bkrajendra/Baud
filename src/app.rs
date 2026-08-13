// src/app.rs
use crate::format::{format_timestamp, LineEnding};
use crate::linebuf::LineAssembler;
use crate::serial::{list_ports, SerialConnection, SerialEvent};
use std::time::{Duration, Instant};

const BAUD_RATES: [u32; 6] = [9600, 19200, 38400, 57600, 115200, 230400];
const PORT_POLL_INTERVAL: Duration = Duration::from_millis(1000);
const NEW_PORT_TOAST_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone)]
struct NewPortToast {
    port: String,
    shown_at: Instant,
}

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

    last_port_scan: Instant,
    new_port_toast: Option<NewPortToast>,
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
            last_port_scan: Instant::now(),
            new_port_toast: None,
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

    fn poll_ports(&mut self) {
        if self.last_port_scan.elapsed() < PORT_POLL_INTERVAL {
            return;
        }
        self.last_port_scan = Instant::now();

        let refreshed = list_ports();
        let new_port = refreshed
            .iter()
            .find(|p| !self.available_ports.contains(p))
            .cloned();

        self.available_ports = refreshed;

        if let Some(port) = new_port {
            self.new_port_toast = Some(NewPortToast {
                port,
                shown_at: Instant::now(),
            });
        }
    }

    fn switch_to_new_port(&mut self, port: String) {
        if self.is_connected() {
            self.disconnect();
        }
        self.selected_port = Some(port);
        self.connect();
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

    fn show_new_port_toast(&mut self, ctx: &egui::Context) {
        let Some(toast) = self.new_port_toast.clone() else {
            return;
        };

        let mut keep_open = true;
        egui::Area::new(egui::Id::new("new_port_toast"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(260.0);
                    ui.strong("New device detected");
                    ui.label(&toast.port);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let label = if self.is_connected() { "Switch" } else { "Connect" };
                        if ui.button(label).clicked() {
                            self.switch_to_new_port(toast.port.clone());
                            keep_open = false;
                        }
                        if ui.button("Dismiss").clicked() {
                            keep_open = false;
                        }
                    });
                });
            });

        if !keep_open {
            self.new_port_toast = None;
        }
    }
}

impl eframe::App for BaudApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;

        self.drain_serial_events();
        self.poll_ports();

        if let Some(toast) = &self.new_port_toast
            && toast.shown_at.elapsed() > NEW_PORT_TOAST_TIMEOUT
        {
            self.new_port_toast = None;
        }

        ctx.request_repaint_after(if self.is_connected() {
            Duration::from_millis(50)
        } else if self.new_port_toast.is_some() {
            Duration::from_millis(200)
        } else {
            PORT_POLL_INTERVAL
        });

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

            let mut scroll_area = egui::ScrollArea::vertical().auto_shrink([false, true]);
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

        self.show_new_port_toast(ctx);
    }
}
