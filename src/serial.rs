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
