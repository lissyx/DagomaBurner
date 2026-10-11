//! Serial port access for the frontend, replacing the native `serialport` Node
//! module. `src/lib/serialport.js` wraps these commands in the subset of the
//! serialport v3 API that the device manager uses.

use std::collections::{HashMap, HashSet};
use std::io::{ErrorKind, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::Serialize;
use serialport::{SerialPort, SerialPortType};
use tauri::ipc::Channel;
use tauri::State;

/// How long the reader thread blocks before checking whether it should stop.
const READ_TIMEOUT: Duration = Duration::from_millis(50);
const WRITE_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortInfo {
    com_name: String,
    manufacturer: Option<String>,
    serial_number: Option<String>,
    vendor_id: Option<u16>,
    product_id: Option<u16>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "camelCase")]
pub enum SerialEvent {
    /// A line received from the device, without its delimiter.
    Data(String),
    /// The device went away (e.g. was unplugged); carries the I/O error.
    Disconnected(String),
}

struct OpenPort {
    writer: Box<dyn SerialPort>,
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
}

/// The ports currently opened by the frontend, keyed by path.
#[derive(Clone, Default)]
pub struct SerialPorts(Arc<Mutex<HashMap<String, OpenPort>>>);

impl SerialPorts {
    /// Closes `path` if it is open, and waits for its reader thread to release the device.
    pub fn close(&self, path: &str) {
        let port = self.0.lock().unwrap().remove(path);
        if let Some(mut port) = port {
            port.stop.store(true, Ordering::Relaxed);
            drop(port.writer);
            if let Some(reader) = port.reader.take() {
                let _ = reader.join();
            }
        }
    }

    fn with_port<T>(
        &self,
        path: &str,
        f: impl FnOnce(&mut dyn SerialPort) -> serialport::Result<T>,
    ) -> Result<T, String> {
        let mut ports = self.0.lock().unwrap();
        let port = ports
            .get_mut(path)
            .ok_or_else(|| format!("Port {path} is not open"))?;
        f(port.writer.as_mut()).map_err(|e| e.to_string())
    }
}

fn read_lines(
    path: String,
    mut port: Box<dyn SerialPort>,
    delimiter: u8,
    stop: Arc<AtomicBool>,
    ports: SerialPorts,
    on_event: Channel<SerialEvent>,
) {
    let mut buf = [0u8; 1024];
    let mut line = Vec::new();

    while !stop.load(Ordering::Relaxed) {
        match port.read(&mut buf) {
            Ok(n) => {
                for &byte in &buf[..n] {
                    if byte == delimiter {
                        let text = String::from_utf8_lossy(&line).into_owned();
                        let _ = on_event.send(SerialEvent::Data(text));
                        line.clear();
                    } else {
                        line.push(byte);
                    }
                }
            }
            Err(e) if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::Interrupted) => {}
            Err(e) => {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                // Forget the port, unless it was already closed and reopened by someone else.
                let mut open = ports.0.lock().unwrap();
                if open.get(&path).is_some_and(|p| Arc::ptr_eq(&p.stop, &stop)) {
                    open.remove(&path);
                }
                drop(open);
                let _ = on_event.send(SerialEvent::Disconnected(e.to_string()));
                break;
            }
        }
    }
}

/// On macOS each USB serial device shows up as both /dev/tty.* and /dev/cu.*;
/// only the callout device is useful here.
fn is_duplicate_tty(name: &str, names: &HashSet<String>) -> bool {
    cfg!(target_os = "macos")
        && name
            .strip_prefix("/dev/tty.")
            .is_some_and(|rest| names.contains(&format!("/dev/cu.{rest}")))
}

#[tauri::command]
pub async fn serial_list() -> Result<Vec<PortInfo>, String> {
    let ports = serialport::available_ports().map_err(|e| e.to_string())?;
    let names: HashSet<String> = ports.iter().map(|p| p.port_name.clone()).collect();

    Ok(ports
        .into_iter()
        .filter(|p| !is_duplicate_tty(&p.port_name, &names))
        .map(|p| match p.port_type {
            SerialPortType::UsbPort(usb) => PortInfo {
                com_name: p.port_name,
                manufacturer: usb.manufacturer,
                serial_number: usb.serial_number,
                vendor_id: Some(usb.vid),
                product_id: Some(usb.pid),
            },
            _ => PortInfo {
                com_name: p.port_name,
                manufacturer: None,
                serial_number: None,
                vendor_id: None,
                product_id: None,
            },
        })
        .collect())
}

#[tauri::command]
pub async fn serial_open(
    ports: State<'_, SerialPorts>,
    path: String,
    baud_rate: u32,
    delimiter: String,
    on_event: Channel<SerialEvent>,
) -> Result<(), String> {
    let delimiter = *delimiter
        .as_bytes()
        .first()
        .ok_or("The line delimiter can't be empty")?;

    let mut open = ports.0.lock().unwrap();
    if open.contains_key(&path) {
        // Same wording as serialport, which the device code checks for.
        return Err("Port is already open".into());
    }

    let reader = serialport::new(&path, baud_rate)
        .timeout(READ_TIMEOUT)
        .open()
        .map_err(|e| e.to_string())?;
    let mut writer = reader.try_clone().map_err(|e| e.to_string())?;
    writer
        .set_timeout(WRITE_TIMEOUT)
        .map_err(|e| e.to_string())?;

    let stop = Arc::new(AtomicBool::new(false));
    let handle = {
        let (path, stop, ports) = (path.clone(), stop.clone(), ports.inner().clone());
        thread::Builder::new()
            .name(format!("serial reader {path}"))
            .spawn(move || read_lines(path, reader, delimiter, stop, ports, on_event))
            .map_err(|e| e.to_string())?
    };

    open.insert(
        path,
        OpenPort {
            writer,
            stop,
            reader: Some(handle),
        },
    );
    Ok(())
}

#[tauri::command]
pub fn serial_write(
    ports: State<'_, SerialPorts>,
    path: String,
    data: String,
) -> Result<(), String> {
    ports.with_port(&path, |port| Ok(port.write_all(data.as_bytes())?))
}

#[tauri::command]
pub fn serial_set(
    ports: State<'_, SerialPorts>,
    path: String,
    rts: bool,
    dtr: bool,
) -> Result<(), String> {
    ports.with_port(&path, |port| {
        port.write_request_to_send(rts)?;
        port.write_data_terminal_ready(dtr)
    })
}

#[tauri::command]
pub async fn serial_close(ports: State<'_, SerialPorts>, path: String) -> Result<(), String> {
    let ports = ports.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ports.close(&path))
        .await
        .map_err(|e| e.to_string())
}
