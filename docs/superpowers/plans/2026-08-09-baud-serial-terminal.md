# Baud Serial Terminal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working v1 of Baud, a minimal egui-based GUI serial terminal for debugging IoT devices — port/baud selection, connect/disconnect, scrolling log view with optional timestamps, text send with configurable line ending, dark/light mode.

**Architecture:** `eframe`/`egui` single-window app. A background thread owns the open serial port and performs blocking reads, forwarding chunks to the UI thread over an `mpsc::channel`. The UI thread drains the channel each frame, assembles bytes into lines via a pure `LineAssembler`, and renders. Writes (sending data) happen synchronously from the UI thread against a cloned/shared port handle. Pure formatting/parsing logic (line endings, timestamps, line assembly) lives in standalone modules with unit tests; the serial I/O and egui wiring are verified by building and running the app, with a final manual hardware test against a real device.

**Tech Stack:** Rust (edition 2024), `eframe`/`egui` (GUI), `serialport` (cross-platform serial I/O), `chrono` (timestamp formatting).

## Global Constraints

- Project lives at `H:/Projects/semon/baud`, package name `baud`, Rust edition 2024 (from spec: existing `Cargo.toml`).
- No settings persistence in v1 — every launch starts at defaults: no port selected, baud 115200, dark mode, timestamps off, autoscroll on, line ending `\n` (from spec's "Out-of-scope clarifications").
- No hex view, no file logging/export, no auto-reconnect, no multi-connection, no macros/history, no scripting (from spec's "Non-goals (v1)").
- Manual test target: real device connected at **COM9** during development (from spec's "Testing Approach").

---

### Task 1: Add dependencies and verify a blank egui window launches

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs` (currently the default `cargo init` hello-world)

**Interfaces:**
- Produces: a runnable `baud` binary that opens an empty eframe/egui window titled "Baud". Later tasks replace the app struct defined here.

- [ ] **Step 1: Add dependencies via cargo add**

Run from `H:/Projects/semon/baud`:

```bash
cargo add eframe
cargo add serialport
cargo add chrono --no-default-features --features clock
```

- [ ] **Step 2: Replace `src/main.rs` with a minimal eframe app**

```rust
// src/main.rs
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Baud",
        options,
        Box::new(|_cc| Ok(Box::new(EmptyApp))),
    )
}

struct EmptyApp;

impl eframe::App for EmptyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Baud starting up...");
        });
    }
}
```

- [ ] **Step 3: Build and run to verify the window appears**

Run: `cargo run`
Expected: a window titled "Baud" opens showing the text "Baud starting up...". Close the window to exit. If it fails to compile, fix errors before continuing (common issue: `egui` not being a direct dependency — run `cargo add egui` if `use egui::...` doesn't resolve, since `eframe` re-exports it but explicit access may need the direct crate depending on version).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs
git commit -m "Add eframe/serialport/chrono deps, launch blank window"
```

---

### Task 2: Pure formatting logic — line endings and timestamps

**Files:**
- Create: `src/format.rs`
- Modify: `src/main.rs` (add `mod format;`)

**Interfaces:**
- Produces:
  - `pub enum LineEnding { None, Lf, Cr, CrLf }` with `pub const ALL: [LineEnding; 4]` and `impl std::fmt::Display for LineEnding` (labels: "None", "\\n", "\\r", "\\r\\n")
  - `pub fn LineEnding::as_bytes(&self) -> &'static [u8]`
  - `pub fn format_timestamp(now: chrono::DateTime<chrono::Local>) -> String` returning `HH:MM:SS.mmm` (e.g. `"14:03:22.451"`)
- Consumed by: Task 3 (sending data) and Task 4 (UI rendering + input bar).

- [ ] **Step 1: Write failing tests**

```rust
// src/format.rs
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn line_ending_bytes() {
        assert_eq!(LineEnding::None.as_bytes(), b"");
        assert_eq!(LineEnding::Lf.as_bytes(), b"\n");
        assert_eq!(LineEnding::Cr.as_bytes(), b"\r");
        assert_eq!(LineEnding::CrLf.as_bytes(), b"\r\n");
    }

    #[test]
    fn line_ending_display_labels() {
        assert_eq!(LineEnding::None.to_string(), "None");
        assert_eq!(LineEnding::Lf.to_string(), "\\n");
        assert_eq!(LineEnding::Cr.to_string(), "\\r");
        assert_eq!(LineEnding::CrLf.to_string(), "\\r\\n");
    }

    #[test]
    fn timestamp_formats_hh_mm_ss_millis() {
        let dt = chrono::Local
            .with_ymd_and_hms(2026, 8, 9, 14, 3, 22)
            .unwrap()
            + chrono::Duration::milliseconds(451);
        assert_eq!(format_timestamp(dt), "14:03:22.451");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail (module doesn't exist yet)**

Run: `cargo test format::`
Expected: FAIL to compile — `format` module / types not defined.

- [ ] **Step 3: Implement `src/format.rs`**

```rust
// src/format.rs (implementation, above the #[cfg(test)] block already written)
use chrono::{DateTime, Local};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    None,
    Lf,
    Cr,
    CrLf,
}

impl LineEnding {
    pub const ALL: [LineEnding; 4] = [
        LineEnding::None,
        LineEnding::Lf,
        LineEnding::Cr,
        LineEnding::CrLf,
    ];

    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            LineEnding::None => b"",
            LineEnding::Lf => b"\n",
            LineEnding::Cr => b"\r",
            LineEnding::CrLf => b"\r\n",
        }
    }
}

impl std::fmt::Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            LineEnding::None => "None",
            LineEnding::Lf => "\\n",
            LineEnding::Cr => "\\r",
            LineEnding::CrLf => "\\r\\n",
        };
        write!(f, "{label}")
    }
}

pub fn format_timestamp(now: DateTime<Local>) -> String {
    now.format("%H:%M:%S%.3f").to_string()
}
```

Add `mod format;` near the top of `src/main.rs` (module is used by later tasks, so an `#[allow(dead_code)]` on the module or its items is fine for now if the compiler warns — do not silence warnings by removing tests or code).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test format::`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/format.rs src/main.rs
git commit -m "Add LineEnding and timestamp formatting with tests"
```

---

### Task 3: Pure line-assembly logic

**Files:**
- Create: `src/linebuf.rs`
- Modify: `src/main.rs` (add `mod linebuf;`)

**Interfaces:**
- Produces:
  - `pub struct LineAssembler { .. }` with `pub fn new() -> Self`, `pub fn push(&mut self, bytes: &[u8]) -> Vec<String>` (returns zero or more newly-completed lines, decoding bytes as UTF-8 lossily, splitting on `\n` and stripping a trailing `\r`), and `pub fn flush(&mut self) -> Option<String>` (returns any partial trailing line as a final "line", or `None` if the buffer is empty — used on disconnect).
- Consumed by: Task 4 (UI thread drains serial-read channel messages through this to produce display lines).

- [ ] **Step 1: Write failing tests**

```rust
// src/linebuf.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_chunk_single_line() {
        let mut a = LineAssembler::new();
        assert_eq!(a.push(b"hello\n"), vec!["hello".to_string()]);
    }

    #[test]
    fn splits_multiple_lines_in_one_chunk() {
        let mut a = LineAssembler::new();
        assert_eq!(
            a.push(b"one\ntwo\nthree\n"),
            vec!["one".to_string(), "two".to_string(), "three".to_string()]
        );
    }

    #[test]
    fn strips_trailing_cr() {
        let mut a = LineAssembler::new();
        assert_eq!(a.push(b"hello\r\n"), vec!["hello".to_string()]);
    }

    #[test]
    fn buffers_partial_line_across_chunks() {
        let mut a = LineAssembler::new();
        assert_eq!(a.push(b"hel"), Vec::<String>::new());
        assert_eq!(a.push(b"lo\n"), vec!["hello".to_string()]);
    }

    #[test]
    fn flush_returns_partial_line() {
        let mut a = LineAssembler::new();
        assert_eq!(a.push(b"partial"), Vec::<String>::new());
        assert_eq!(a.flush(), Some("partial".to_string()));
        assert_eq!(a.flush(), None);
    }

    #[test]
    fn invalid_utf8_is_lossily_decoded() {
        let mut a = LineAssembler::new();
        let lines = a.push(&[0xFF, 0xFE, b'\n']);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains('\u{FFFD}'));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test linebuf::`
Expected: FAIL to compile — `linebuf` module not defined.

- [ ] **Step 3: Implement `src/linebuf.rs`**

```rust
// src/linebuf.rs (implementation, above the #[cfg(test)] block already written)
pub struct LineAssembler {
    partial: Vec<u8>,
}

impl LineAssembler {
    pub fn new() -> Self {
        Self { partial: Vec::new() }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.partial.extend_from_slice(bytes);

        let mut lines = Vec::new();
        while let Some(pos) = self.partial.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = self.partial.drain(..=pos).collect();
            line.pop(); // remove '\n'
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            lines.push(String::from_utf8_lossy(&line).into_owned());
        }
        lines
    }

    pub fn flush(&mut self) -> Option<String> {
        if self.partial.is_empty() {
            return None;
        }
        let remaining = std::mem::take(&mut self.partial);
        Some(String::from_utf8_lossy(&remaining).into_owned())
    }
}
```

Add `mod linebuf;` near the top of `src/main.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test linebuf::`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add src/linebuf.rs src/main.rs
git commit -m "Add LineAssembler for splitting serial byte stream into lines"
```

---

### Task 4: Serial connection module (background thread + channel)

**Files:**
- Create: `src/serial.rs`
- Modify: `src/main.rs` (add `mod serial;`)

**Interfaces:**
- Consumes: nothing from earlier tasks (uses `serialport` crate directly).
- Produces:
  - `pub fn list_ports() -> Vec<String>` — sorted port names from `serialport::available_ports()`.
  - `pub enum SerialEvent { Data(Vec<u8>), Error(String), Closed }`
  - `pub struct SerialConnection { .. }` with:
    - `pub fn open(port_name: &str, baud: u32) -> Result<Self, String>` — opens the port with a short read timeout (e.g. 100ms so the reader thread can poll a stop flag), spawns the background reader thread, returns the connection or an `Err(String)` describing why open failed.
    - `pub fn events(&self) -> &std::sync::mpsc::Receiver<SerialEvent>` — channel the UI drains each frame.
    - `pub fn send(&self, bytes: &[u8]) -> Result<(), String>` — writes to the port synchronously.
    - `pub fn close(self)` — signals the reader thread to stop and joins it, consuming `self` so the port is fully released on return.
  - Consumed by: Task 5 (`app.rs` holds an `Option<SerialConnection>` and drains `events()` each frame).

- [ ] **Step 1: Implement `src/serial.rs`**

This module talks to real hardware, so it is verified by build + the manual
hardware test in Task 6 rather than by unit tests (per the spec's Testing
Approach — serial I/O isn't practical to unit test).

```rust
// src/serial.rs
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

pub fn list_ports() -> Vec<String> {
    let mut names: Vec<String> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.port_name)
        .collect();
    names.sort();
    names
}

pub enum SerialEvent {
    Data(Vec<u8>),
    Error(String),
    Closed,
}

pub struct SerialConnection {
    write_port: Box<dyn serialport::SerialPort>,
    events_rx: Receiver<SerialEvent>,
    stop_flag: Arc<AtomicBool>,
    reader_thread: Option<JoinHandle<()>>,
}

impl SerialConnection {
    pub fn open(port_name: &str, baud: u32) -> Result<Self, String> {
        let read_port = serialport::new(port_name, baud)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| format!("Failed to open {port_name}: {e}"))?;

        let write_port = read_port
            .try_clone()
            .map_err(|e| format!("Failed to prepare {port_name} for writing: {e}"))?;

        let (tx, rx) = mpsc::channel();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let thread_stop_flag = stop_flag.clone();

        let reader_thread = std::thread::spawn(move || {
            let mut port = read_port;
            let mut buf = [0u8; 1024];
            loop {
                if thread_stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                match port.read(&mut buf) {
                    Ok(0) => continue,
                    Ok(n) => {
                        if tx.send(SerialEvent::Data(buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::TimedOut => continue,
                    Err(e) => {
                        let _ = tx.send(SerialEvent::Error(e.to_string()));
                        let _ = tx.send(SerialEvent::Closed);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            write_port,
            events_rx: rx,
            stop_flag,
            reader_thread: Some(reader_thread),
        })
    }

    pub fn events(&self) -> &Receiver<SerialEvent> {
        &self.events_rx
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.write_port
            .write_all(bytes)
            .map_err(|e| format!("Write failed: {e}"))
    }

    pub fn close(mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }
    }
}
```

Add `mod serial;` near the top of `src/main.rs`.

- [ ] **Step 2: Build to verify it compiles**

Run: `cargo build`
Expected: builds with no errors (unused-code warnings are fine at this
stage since `app.rs`/`main.rs` don't wire it in yet — do not add `#[allow(dead_code)]`
suppressions; Task 5 will consume these items and the warnings will clear
on their own).

- [ ] **Step 3: Commit**

```bash
git add src/serial.rs src/main.rs
git commit -m "Add SerialConnection: background reader thread + event channel"
```

---

### Task 5: Wire up the full egui UI (app.rs)

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs` (replace `EmptyApp` from Task 1 with `app::BaudApp`, add `mod app;`)

**Interfaces:**
- Consumes:
  - `format::LineEnding` (+ `ALL`, `as_bytes`, `Display`), `format::format_timestamp`
  - `linebuf::LineAssembler::{new, push, flush}`
  - `serial::{list_ports, SerialConnection, SerialEvent}`
- Produces: `pub struct BaudApp` implementing `eframe::App`, used by `main.rs` as the app passed to `eframe::run_native`.

- [ ] **Step 1: Implement `src/app.rs`**

```rust
// src/app.rs
use crate::format::{format_timestamp, LineEnding};
use crate::linebuf::LineAssembler;
use crate::serial::{list_ports, SerialConnection, SerialEvent};

const BAUD_RATES: [u32; 6] = [9600, 19200, 38400, 57600, 115200, 230400];

pub struct BaudApp {
    available_ports: Vec<String>,
    selected_port: Option<String>,
    baud_rate: u32,
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
        let mut disconnected = false;
        if let Some(conn) = &self.connection {
            while let Ok(event) = conn.events().try_recv() {
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_serial_events();
        if self.is_connected() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        ctx.set_visuals(if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
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
                        .selected_text(self.baud_rate.to_string())
                        .show_ui(ui, |ui| {
                            for rate in BAUD_RATES {
                                ui.selectable_value(&mut self.baud_rate, rate, rate.to_string());
                            }
                        });

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

        egui::TopBottomPanel::bottom("input_bar").show(ctx, |ui| {
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

        egui::CentralPanel::default().show(ctx, |ui| {
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
```

- [ ] **Step 2: Update `src/main.rs` to use `BaudApp`**

```rust
// src/main.rs
mod app;
mod format;
mod linebuf;
mod serial;

use app::BaudApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Baud",
        options,
        Box::new(|_cc| Ok(Box::new(BaudApp::default()))),
    )
}
```

- [ ] **Step 3: Build and run**

Run: `cargo build`
Expected: compiles cleanly (fix any egui API mismatches against the
`eframe`/`egui` version that `cargo add` resolved in Task 1 — method names
like `stick_to_bottom`, `selectable_value`, and `ComboBox::from_id_salt`
match egui 0.29+; if an older/newer version resolved, adjust to that
version's equivalent API, e.g. `from_id_source` on older egui).

Run: `cargo run`
Expected: window opens with port dropdown, baud dropdown, Connect button,
timestamp/dark-mode checkboxes, empty scrollable terminal area with
Autoscroll/Clear controls, and a bottom input bar with a line-ending
dropdown and Send button (input disabled while disconnected). Close the
window to exit.

- [ ] **Step 4: Run full test suite to confirm no regressions**

Run: `cargo test`
Expected: PASS (the 3 `format::` tests and 6 `linebuf::` tests from Tasks
2–3 still pass; `app.rs`/`serial.rs` have no unit tests by design).

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "Wire up full Baud UI: port/baud selection, terminal view, send bar"
```

---

### Task 6: Manual hardware verification against the device on COM9

**Files:** none (manual verification task, no code changes expected unless
a bug is found).

**Interfaces:** none — this task exercises the full app built in Tasks 1–5.

- [ ] **Step 1: Launch the app**

Run: `cargo run --release`

- [ ] **Step 2: Connect to the device**

In the running app: click the port dropdown, confirm `COM9` appears in the
list (if not, click Refresh once); select it; confirm baud rate matches the
device's configured rate (try 115200 first); click Connect.

Expected: no error banner appears; the Connect button changes to
Disconnect; the port/baud controls become disabled while connected.

- [ ] **Step 3: Verify incoming data display**

Expected: if the device emits any serial output, lines appear in the
terminal pane in near-real-time, auto-scrolling to the bottom. Toggle
"Timestamps" on and off and confirm each line gains/loses a
`[HH:MM:SS.mmm]` prefix.

- [ ] **Step 4: Verify sending data**

Type a short command the device recognizes (or any text if just checking
transmission) into the input field and press Enter (and separately, try
the Send button). Try switching the line-ending dropdown between `\n`,
`\r\n`, and `None` and confirm the device's behavior changes accordingly
(e.g., a device expecting `\r\n` line termination only responds correctly
in that mode) — this confirms bytes are actually being written with the
selected terminator.

- [ ] **Step 5: Verify Clear and Autoscroll**

Click Clear and confirm the terminal view empties. Turn off Autoscroll,
let more data arrive (or send a few commands that produce responses),
and confirm the view does not jump to the bottom; scroll up manually to
confirm history is retained.

- [ ] **Step 6: Verify disconnect/reconnect**

Click Disconnect. Expected: input bar disables, port/baud controls
re-enable. Click Connect again on the same port. Expected: reconnects
successfully with no error (confirms the port was fully released on
disconnect).

- [ ] **Step 7: Verify dark/light toggle**

Toggle "Dark mode" off and on; confirm the whole UI's color scheme
switches accordingly.

- [ ] **Step 8: Record results**

If every check in Steps 2–7 passes, the v1 demo is verified working
end-to-end against real hardware — no commit needed for this task. If any
step fails, treat it as a bug: reproduce, fix the relevant module from
Tasks 2–5, re-run the failing step, and commit the fix with a message
describing the bug and fix.

---

## Self-Review Notes

- Spec coverage checked: port auto-detect + refresh (Task 5), baud dropdown
  + custom entry (Task 5), connect/disconnect (Tasks 4–5), timestamp toggle
  (Tasks 2, 5), dark/light toggle (Task 5), autoscroll + clear (Task 5),
  line-ending dropdown + send (Tasks 2, 5), error banner on failed
  connect/read error (Tasks 4–5), clean port release on disconnect (Task 4),
  manual hardware test on COM9 (Task 6) — all covered.
- Non-goals from the spec (hex view, file export, auto-reconnect,
  multi-connection, macros/history/scripting, settings persistence) are
  intentionally absent from every task above.
