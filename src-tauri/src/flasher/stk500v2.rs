//! STK500 version 2 protocol, spoken by the ATmega2560 (Wiring) bootloader.

use std::time::{Duration, Instant};

use super::{check_signature, read_exact, Board, Link};

const MESSAGE_START: u8 = 0x1B;
const TOKEN: u8 = 0x0E;

const CMD_SIGN_ON: u8 = 0x01;
const CMD_LOAD_ADDRESS: u8 = 0x06;
const CMD_ENTER_PROGMODE_ISP: u8 = 0x10;
const CMD_LEAVE_PROGMODE_ISP: u8 = 0x11;
const CMD_PROGRAM_FLASH_ISP: u8 = 0x13;
const CMD_READ_FLASH_ISP: u8 = 0x14;
const CMD_READ_SIGNATURE_ISP: u8 = 0x1B;
const STATUS_CMD_OK: u8 = 0x00;

const SIGN_ON_ATTEMPTS: usize = 10;
/// Flash beyond 128 KiB needs the "extended address" bit in CMD_LOAD_ADDRESS.
const LOAD_EXTENDED_ADDRESS: u32 = 0x8000_0000;

struct Stk500v2<'a> {
    link: &'a mut dyn Link,
    timeout: Duration,
    sequence: u8,
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |sum, b| sum ^ b)
}

impl Stk500v2<'_> {
    /// Sends a message `body` and returns the answer body, checking that it
    /// echoes the command with an OK status.
    fn command(&mut self, body: &[u8]) -> Result<Vec<u8>, String> {
        let mut message = vec![MESSAGE_START, self.sequence];
        message.extend_from_slice(&(body.len() as u16).to_be_bytes());
        message.push(TOKEN);
        message.extend_from_slice(body);
        message.push(checksum(&message));
        self.link.write_all(&message).map_err(|e| e.to_string())?;

        let answer = self
            .receive()
            .map_err(|e| format!("command {:#04x}: {e}", body[0]))?;
        self.sequence = self.sequence.wrapping_add(1);
        if answer.len() < 2 || answer[0] != body[0] || answer[1] != STATUS_CMD_OK {
            return Err(format!(
                "command {:#04x}: unexpected answer {answer:02x?}",
                body[0]
            ));
        }
        Ok(answer)
    }

    fn receive(&mut self) -> Result<Vec<u8>, String> {
        let deadline = Instant::now() + self.timeout;
        // Skip anything (e.g. firmware output) preceding the message.
        while read_exact(self.link, 1, deadline)?[0] != MESSAGE_START {}

        let header = read_exact(self.link, 4, deadline)?;
        let size = u16::from_be_bytes([header[1], header[2]]) as usize;
        let body = read_exact(self.link, size, deadline)?;
        let received_checksum = read_exact(self.link, 1, deadline)?[0];

        if header[0] != self.sequence {
            return Err(format!(
                "answer to message {} instead of {}",
                header[0], self.sequence
            ));
        }
        if header[3] != TOKEN {
            return Err("malformed answer".into());
        }
        if checksum(&[&[MESSAGE_START][..], &header, &body].concat()) != received_checksum {
            return Err("bad answer checksum".into());
        }
        Ok(body)
    }

    fn sign_on(&mut self) -> Result<(), String> {
        let mut last_error = String::new();
        for _ in 0..SIGN_ON_ATTEMPTS {
            self.link.clear_input();
            match self.command(&[CMD_SIGN_ON]) {
                Ok(_) => return Ok(()),
                Err(e) => last_error = e,
            }
        }
        Err(format!("can't sync with the bootloader: {last_error}"))
    }

    fn load_address(&mut self, byte_address: usize, extended: bool) -> Result<(), String> {
        let mut word_address = (byte_address >> 1) as u32;
        if extended {
            word_address |= LOAD_EXTENDED_ADDRESS;
        }
        let mut body = vec![CMD_LOAD_ADDRESS];
        body.extend_from_slice(&word_address.to_be_bytes());
        self.command(&body).map(drop)
    }
}

pub fn flash(link: &mut dyn Link, board: &Board, image: &[u8]) -> Result<(), String> {
    let mut stk = Stk500v2 {
        link,
        timeout: board.response_timeout,
        sequence: 1,
    };
    let extended = image.len() > 0x2_0000;

    stk.sign_on()?;
    let mut signature = Vec::new();
    for index in 0..3 {
        let answer = stk.command(&[CMD_READ_SIGNATURE_ISP, 4, 0x30, 0x00, index, 0x00])?;
        signature.push(*answer.get(2).ok_or("short signature answer")?);
    }
    check_signature(board, &signature)?;

    stk.command(&[
        CMD_ENTER_PROGMODE_ISP,
        board.isp_timeout,
        board.stab_delay,
        board.cmdexe_delay,
        board.synch_loops,
        board.byte_delay,
        board.poll_value,
        board.poll_index,
        0xAC,
        0x53,
        0x00,
        0x00,
    ])?;

    for (index, page) in image.chunks(board.page_size).enumerate() {
        stk.load_address(index * board.page_size, extended)?;
        let mut body = vec![CMD_PROGRAM_FLASH_ISP];
        body.extend_from_slice(&(page.len() as u16).to_be_bytes());
        // Page mode write, then the ISP opcodes (load page, write page, read).
        body.extend_from_slice(&[0xC1, board.delay1, 0x40, 0x4C, 0x20, 0x00, 0x00]);
        body.extend_from_slice(page);
        stk.command(&body)?;
    }

    for (index, page) in image.chunks(board.page_size).enumerate() {
        let address = index * board.page_size;
        stk.load_address(address, extended)?;
        let [hi, lo] = (page.len() as u16).to_be_bytes();
        let answer = stk.command(&[CMD_READ_FLASH_ISP, hi, lo, 0x20])?;
        // Answer: command, status, data..., status.
        if answer.get(2..2 + page.len()) != Some(page) {
            return Err(format!("verification failed in the page at {address:#07x}"));
        }
    }

    stk.command(&[CMD_LEAVE_PROGMODE_ISP, 0x01, 0x01])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flasher::testing::{sample_image, FakeLink};
    use crate::flasher::MEGA;

    /// A minimal Wiring-like bootloader over `flash`. It prints `noise`
    /// before its first answer, like a firmware that was still running.
    fn bootloader<'a>(
        flash: &'a mut Vec<u8>,
        signature: [u8; 3],
        mut noise: &'static [u8],
        extended_seen: &'a mut bool,
    ) -> impl FnMut(&[u8]) -> (usize, Vec<u8>) + 'a {
        let mut address = 0usize;
        move |msg: &[u8]| {
            if msg.len() < 5 {
                return (0, vec![]);
            }
            assert_eq!(msg[0], MESSAGE_START);
            assert_eq!(msg[4], TOKEN);
            let size = u16::from_be_bytes([msg[2], msg[3]]) as usize;
            if msg.len() < 6 + size {
                return (0, vec![]);
            }
            assert_eq!(checksum(&msg[..5 + size]), msg[5 + size], "checksum");
            let body = &msg[5..5 + size];

            let answer = match body[0] {
                CMD_SIGN_ON => [&[CMD_SIGN_ON, STATUS_CMD_OK, 8][..], b"AVRISP_2"].concat(),
                CMD_READ_SIGNATURE_ISP => {
                    vec![
                        CMD_READ_SIGNATURE_ISP,
                        STATUS_CMD_OK,
                        signature[body[4] as usize],
                        STATUS_CMD_OK,
                    ]
                }
                CMD_LOAD_ADDRESS => {
                    let word = u32::from_be_bytes([body[1], body[2], body[3], body[4]]);
                    *extended_seen |= word & LOAD_EXTENDED_ADDRESS != 0;
                    address = ((word & !LOAD_EXTENDED_ADDRESS) as usize) * 2;
                    vec![CMD_LOAD_ADDRESS, STATUS_CMD_OK]
                }
                CMD_PROGRAM_FLASH_ISP => {
                    let len = u16::from_be_bytes([body[1], body[2]]) as usize;
                    let data = &body[10..];
                    assert_eq!(data.len(), len);
                    if flash.len() < address + len {
                        flash.resize(address + len, 0xFF);
                    }
                    flash[address..address + len].copy_from_slice(data);
                    vec![CMD_PROGRAM_FLASH_ISP, STATUS_CMD_OK]
                }
                CMD_READ_FLASH_ISP => {
                    let len = u16::from_be_bytes([body[1], body[2]]) as usize;
                    [
                        &[CMD_READ_FLASH_ISP, STATUS_CMD_OK][..],
                        &flash[address..address + len],
                        &[STATUS_CMD_OK],
                    ]
                    .concat()
                }
                cmd @ (CMD_ENTER_PROGMODE_ISP | CMD_LEAVE_PROGMODE_ISP) => vec![cmd, STATUS_CMD_OK],
                other => panic!("unexpected command {other:#04x}"),
            };

            let mut reply = std::mem::take(&mut noise).to_vec();
            let mut frame = vec![MESSAGE_START, msg[1]];
            frame.extend_from_slice(&(answer.len() as u16).to_be_bytes());
            frame.push(TOKEN);
            frame.extend_from_slice(&answer);
            frame.push(checksum(&frame));
            reply.extend(frame);
            (6 + size, reply)
        }
    }

    fn flash_image(
        image: &[u8],
        flash: &mut Vec<u8>,
        signature: [u8; 3],
        extended_seen: &mut bool,
    ) -> Result<(), String> {
        let board = Board {
            response_timeout: Duration::from_millis(5),
            ..MEGA
        };
        let mut link = FakeLink::new(bootloader(
            flash,
            signature,
            b"echo:Marlin\n",
            extended_seen,
        ));
        super::flash(&mut link, &board, image)
    }

    #[test]
    fn writes_and_verifies_image() {
        let image = sample_image(5 * 256 + 17);
        let (mut flash, mut extended) = (Vec::new(), false);
        flash_image(&image, &mut flash, MEGA.signature, &mut extended).unwrap();
        assert_eq!(flash, image);
        assert!(!extended);
    }

    #[test]
    fn uses_extended_addresses_beyond_128k() {
        let image = sample_image(0x2_0000 + 256);
        let (mut flash, mut extended) = (Vec::new(), false);
        flash_image(&image, &mut flash, MEGA.signature, &mut extended).unwrap();
        assert_eq!(flash, image);
        assert!(extended);
    }

    #[test]
    fn rejects_wrong_signature() {
        let (mut flash, mut extended) = (Vec::new(), false);
        let error = flash_image(
            &sample_image(10),
            &mut flash,
            [0x1e, 0x97, 0x05],
            &mut extended,
        )
        .unwrap_err();
        assert!(error.contains("signature"), "{error}");
        assert!(flash.is_empty());
    }
}
