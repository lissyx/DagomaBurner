//! Firmware upload to the printer boards through their serial bootloaders,
//! replacing avrgirl-arduino.

mod ihex;
mod stk500v1;
mod stk500v2;

use std::io::{ErrorKind, Read, Write};
use std::thread;
use std::time::{Duration, Instant};

use serialport::{ClearBuffer, SerialPort};
use tauri::State;

use crate::serial::SerialPorts;

/// Polling granularity of serial reads; response deadlines are handled on top.
const POLL_TIMEOUT: Duration = Duration::from_millis(50);

#[derive(Debug, PartialEq)]
enum Protocol {
    Stk500v1,
    Stk500v2,
}

/// Board parameters, from avrgirl-arduino's board list and the overrides the
/// app applied to it.
#[derive(Debug)]
pub struct Board {
    name: &'static str,
    baud: u32,
    signature: [u8; 3],
    page_size: usize,
    protocol: Protocol,
    /// How long to wait for each bootloader response.
    response_timeout: Duration,
    // STK500v2 ISP parameters, sent as-is to the bootloader.
    isp_timeout: u8,
    stab_delay: u8,
    cmdexe_delay: u8,
    synch_loops: u8,
    byte_delay: u8,
    poll_value: u8,
    poll_index: u8,
    delay1: u8,
}

/// Melzi boards (Discovery200): ATmega1284P with an STK500v1 bootloader.
const MELZI: Board = Board {
    name: "melzi",
    baud: 57600,
    signature: [0x1e, 0x97, 0x05],
    page_size: 256,
    protocol: Protocol::Stk500v1,
    response_timeout: Duration::from_millis(0x320),
    isp_timeout: 0xc8,
    stab_delay: 0x64,
    cmdexe_delay: 0x19,
    synch_loops: 0x20,
    byte_delay: 0x00,
    poll_value: 0x53,
    poll_index: 0x03,
    delay1: 10,
};

/// Mega 2560 based boards (MKS): ATmega2560 with an STK500v2 bootloader.
const MEGA: Board = Board {
    name: "mega",
    baud: 115200,
    signature: [0x1e, 0x98, 0x01],
    page_size: 256,
    protocol: Protocol::Stk500v2,
    response_timeout: Duration::from_millis(0x320),
    isp_timeout: 0xc8,
    stab_delay: 0x64,
    cmdexe_delay: 0x19,
    synch_loops: 0x20,
    byte_delay: 0x00,
    poll_value: 0x53,
    poll_index: 0x03,
    delay1: 10,
};

fn board_by_name(name: &str) -> Option<&'static Board> {
    [&MELZI, &MEGA].into_iter().find(|b| b.name == name)
}

/// The byte stream the bootloader protocols talk over.
pub trait Link: Read + Write {
    /// Drops any received bytes that haven't been read yet.
    fn clear_input(&mut self);
}

impl Link for Box<dyn SerialPort> {
    fn clear_input(&mut self) {
        let _ = self.clear(ClearBuffer::Input);
    }
}

/// Reads exactly `len` bytes, failing if they don't arrive before `deadline`.
fn read_exact(link: &mut dyn Link, len: usize, deadline: Instant) -> Result<Vec<u8>, String> {
    let mut buf = vec![0; len];
    let mut filled = 0;
    while filled < len {
        match link.read(&mut buf[filled..]) {
            Ok(n) => filled += n,
            Err(e) if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::Interrupted) => {}
            Err(e) => return Err(e.to_string()),
        }
        if filled < len && Instant::now() >= deadline {
            return Err(format!(
                "timed out waiting for the bootloader ({filled}/{len} bytes received)"
            ));
        }
    }
    Ok(buf)
}

fn check_signature(board: &Board, signature: &[u8]) -> Result<(), String> {
    if signature == board.signature {
        Ok(())
    } else {
        Err(format!(
            "unexpected device signature {signature:02x?}, expected {:02x?} for {}",
            board.signature, board.name
        ))
    }
}

/// Pulses DTR/RTS to reset the board into its bootloader, like avrdude's
/// arduino programmer does.
fn reset(port: &mut Box<dyn SerialPort>) -> Result<(), String> {
    let set = |port: &mut Box<dyn SerialPort>, level| {
        port.write_data_terminal_ready(level)
            .and_then(|_| port.write_request_to_send(level))
            .map_err(|e| e.to_string())
    };
    set(port, false)?;
    thread::sleep(Duration::from_millis(250));
    set(port, true)?;
    thread::sleep(Duration::from_millis(50));
    port.clear_input();
    Ok(())
}

fn flash(board: &Board, path: &str, image: &[u8]) -> Result<(), String> {
    let mut port = serialport::new(path, board.baud)
        .timeout(POLL_TIMEOUT)
        .open()
        .map_err(|e| format!("can't open {path}: {e}"))?;
    reset(&mut port)?;
    match board.protocol {
        Protocol::Stk500v1 => stk500v1::flash(&mut port, board, image),
        Protocol::Stk500v2 => stk500v2::flash(&mut port, board, image),
    }
}

/// Uploads an Intel HEX firmware to the board on `port` and verifies it.
#[tauri::command]
pub async fn flash_firmware(
    ports: State<'_, SerialPorts>,
    port: String,
    hex: String,
    board: String,
) -> Result<(), String> {
    let board = board_by_name(&board).ok_or_else(|| format!("unknown board {board}"))?;
    let image = ihex::parse(&hex).map_err(|e| format!("invalid firmware file: {e}"))?;
    let ports = ports.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        // The bootloader needs the port to itself.
        ports.close(&port);
        flash(board, &port, &image)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod testing {
    use std::collections::VecDeque;
    use std::io::{self, Read, Write};

    use super::Link;

    /// A fake serial link to a simulated bootloader: bytes written are fed to
    /// `respond`, and whatever it returns becomes readable.
    pub struct FakeLink<F: FnMut(&[u8]) -> (usize, Vec<u8>)> {
        pub pending: Vec<u8>,
        pub incoming: VecDeque<u8>,
        /// Consumes a prefix of the written bytes, returning how many bytes
        /// made a complete request (0 if more are needed) and the reply.
        pub respond: F,
    }

    impl<F: FnMut(&[u8]) -> (usize, Vec<u8>)> FakeLink<F> {
        pub fn new(respond: F) -> Self {
            FakeLink {
                pending: Vec::new(),
                incoming: VecDeque::new(),
                respond,
            }
        }
    }

    impl<F: FnMut(&[u8]) -> (usize, Vec<u8>)> Read for FakeLink<F> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.incoming.is_empty() {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let n = buf.len().min(self.incoming.len());
            for (slot, byte) in buf.iter_mut().zip(self.incoming.drain(..n)) {
                *slot = byte;
            }
            Ok(n)
        }
    }

    impl<F: FnMut(&[u8]) -> (usize, Vec<u8>)> Write for FakeLink<F> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.pending.extend_from_slice(buf);
            loop {
                let (used, reply) = (self.respond)(&self.pending);
                if used == 0 {
                    break;
                }
                self.pending.drain(..used);
                self.incoming.extend(reply);
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<F: FnMut(&[u8]) -> (usize, Vec<u8>)> Link for FakeLink<F> {
        fn clear_input(&mut self) {
            self.incoming.clear();
        }
    }

    /// A pseudo-random firmware image spanning several pages, with a partial last page.
    pub fn sample_image(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 7 + i / 256) as u8).collect()
    }
}
