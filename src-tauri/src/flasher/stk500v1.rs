//! STK500 version 1 protocol, spoken by the Melzi (Sanguino/Optiboot) bootloaders.

use std::thread;
use std::time::{Duration, Instant};

use super::{check_signature, read_exact, Board, Link};

const STK_OK: u8 = 0x10;
const STK_INSYNC: u8 = 0x14;
const CRC_EOP: u8 = 0x20;
const STK_GET_SYNC: u8 = 0x30;
const STK_SET_DEVICE: u8 = 0x42;
const STK_ENTER_PROGMODE: u8 = 0x50;
const STK_LEAVE_PROGMODE: u8 = 0x51;
const STK_LOAD_ADDRESS: u8 = 0x55;
const STK_PROG_PAGE: u8 = 0x64;
const STK_READ_PAGE: u8 = 0x74;
const STK_READ_SIGN: u8 = 0x75;
const MEMTYPE_FLASH: u8 = b'F';

const SYNC_ATTEMPTS: usize = 10;

struct Stk500v1<'a> {
    link: &'a mut dyn Link,
    timeout: Duration,
}

impl Stk500v1<'_> {
    /// Sends `request` and returns the `response_len` bytes framed by INSYNC/OK.
    fn command(&mut self, request: &[u8], response_len: usize) -> Result<Vec<u8>, String> {
        self.link.write_all(request).map_err(|e| e.to_string())?;
        let response = read_exact(self.link, response_len + 2, Instant::now() + self.timeout)
            .map_err(|e| format!("command {:#04x}: {e}", request[0]))?;
        if response[0] != STK_INSYNC || response[response_len + 1] != STK_OK {
            return Err(format!(
                "command {:#04x}: unexpected response {response:02x?}",
                request[0]
            ));
        }
        Ok(response[1..=response_len].to_vec())
    }

    fn sync(&mut self) -> Result<(), String> {
        let mut last_error = String::new();
        for _ in 0..SYNC_ATTEMPTS {
            self.link.clear_input();
            match self.command(&[STK_GET_SYNC, CRC_EOP], 0) {
                Ok(_) => {
                    // A late answer to an earlier attempt may still be on its
                    // way: let it arrive, drop it, and sync once more.
                    thread::sleep(Duration::from_millis(50));
                    self.link.clear_input();
                    return self.command(&[STK_GET_SYNC, CRC_EOP], 0).map(drop);
                }
                Err(e) => last_error = e,
            }
        }
        Err(format!("can't sync with the bootloader: {last_error}"))
    }

    fn load_address(&mut self, byte_address: usize) -> Result<(), String> {
        let [lo, hi] = ((byte_address >> 1) as u16).to_le_bytes();
        self.command(&[STK_LOAD_ADDRESS, lo, hi, CRC_EOP], 0)
            .map(drop)
    }
}

fn set_device_request(board: &Board) -> Vec<u8> {
    // devicecode, revision, progtype, parmode, polling, selftimed, lockbytes,
    // fusebytes, flashpollval1/2, eeprompollval1/2, pagesize (2), eepromsize (2),
    // flashsize (4). Bootloaders only care about the page size, if anything.
    let mut request = vec![STK_SET_DEVICE];
    request.extend_from_slice(&[0; 12]);
    request.extend_from_slice(&(board.page_size as u16).to_be_bytes());
    request.extend_from_slice(&[0; 6]);
    request.push(CRC_EOP);
    request
}

pub fn flash(link: &mut dyn Link, board: &Board, image: &[u8]) -> Result<(), String> {
    let mut stk = Stk500v1 {
        link,
        timeout: board.response_timeout,
    };

    stk.sync()?;
    let signature = stk.command(&[STK_READ_SIGN, CRC_EOP], 3)?;
    check_signature(board, &signature)?;
    stk.command(&set_device_request(board), 0)?;
    stk.command(&[STK_ENTER_PROGMODE, CRC_EOP], 0)?;

    for (index, page) in image.chunks(board.page_size).enumerate() {
        stk.load_address(index * board.page_size)?;
        let mut request = vec![STK_PROG_PAGE];
        request.extend_from_slice(&(page.len() as u16).to_be_bytes());
        request.push(MEMTYPE_FLASH);
        request.extend_from_slice(page);
        request.push(CRC_EOP);
        stk.command(&request, 0)?;
    }

    for (index, page) in image.chunks(board.page_size).enumerate() {
        let address = index * board.page_size;
        stk.load_address(address)?;
        let [hi, lo] = (page.len() as u16).to_be_bytes();
        let read = stk.command(&[STK_READ_PAGE, hi, lo, MEMTYPE_FLASH, CRC_EOP], page.len())?;
        if read != page {
            return Err(format!("verification failed in the page at {address:#07x}"));
        }
    }

    stk.command(&[STK_LEAVE_PROGMODE, CRC_EOP], 0)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flasher::testing::{sample_image, FakeLink};
    use crate::flasher::MELZI;

    /// A minimal Optiboot-like bootloader over `flash`, answering a full
    /// request at a time. `sync_failures` initial GET_SYNCs go unanswered.
    fn bootloader<'a>(
        flash: &'a mut Vec<u8>,
        signature: [u8; 3],
        mut sync_failures: usize,
    ) -> impl FnMut(&[u8]) -> (usize, Vec<u8>) + 'a {
        let mut address = 0usize;
        move |req: &[u8]| {
            let ok = |data: &[u8]| [&[STK_INSYNC][..], data, &[STK_OK]].concat();
            let Some(&cmd) = req.first() else {
                return (0, vec![]);
            };
            let len = match cmd {
                STK_GET_SYNC | STK_READ_SIGN | STK_ENTER_PROGMODE | STK_LEAVE_PROGMODE => 2,
                STK_LOAD_ADDRESS => 4,
                STK_SET_DEVICE => 22,
                STK_READ_PAGE => 5,
                STK_PROG_PAGE if req.len() >= 3 => {
                    5 + u16::from_be_bytes([req[1], req[2]]) as usize
                }
                STK_PROG_PAGE => return (0, vec![]),
                other => panic!("unexpected command {other:#04x}"),
            };
            if req.len() < len {
                return (0, vec![]);
            }
            assert_eq!(req[len - 1], CRC_EOP, "request {:02x?}", &req[..len]);
            let reply = match cmd {
                STK_GET_SYNC if sync_failures > 0 => {
                    sync_failures -= 1;
                    vec![]
                }
                STK_READ_SIGN => ok(&signature),
                STK_LOAD_ADDRESS => {
                    address = u16::from_le_bytes([req[1], req[2]]) as usize * 2;
                    ok(&[])
                }
                STK_PROG_PAGE => {
                    assert_eq!(req[3], MEMTYPE_FLASH);
                    let data = &req[4..len - 1];
                    if flash.len() < address + data.len() {
                        flash.resize(address + data.len(), 0xFF);
                    }
                    flash[address..address + data.len()].copy_from_slice(data);
                    ok(&[])
                }
                STK_READ_PAGE => {
                    let size = u16::from_be_bytes([req[1], req[2]]) as usize;
                    ok(&flash[address..address + size])
                }
                _ => ok(&[]),
            };
            (len, reply)
        }
    }

    #[test]
    fn writes_and_verifies_image() {
        let image = sample_image(3 * 256 + 100);
        let mut flash = Vec::new();
        flash_image(&image, &mut flash, MELZI.signature, 3).unwrap();
        assert_eq!(flash, image);
    }

    #[test]
    fn rejects_wrong_signature() {
        let mut flash = Vec::new();
        let error = flash_image(&sample_image(10), &mut flash, [0x1e, 0x96, 0x0a], 0).unwrap_err();
        assert!(error.contains("signature"), "{error}");
        assert!(flash.is_empty());
    }

    #[test]
    fn gives_up_when_bootloader_is_silent() {
        let mut flash = Vec::new();
        let error =
            flash_image(&sample_image(10), &mut flash, MELZI.signature, usize::MAX).unwrap_err();
        assert!(error.contains("can't sync"), "{error}");
    }

    fn flash_image(
        image: &[u8],
        flash: &mut Vec<u8>,
        signature: [u8; 3],
        sync_failures: usize,
    ) -> Result<(), String> {
        let board = Board {
            response_timeout: Duration::from_millis(5),
            ..MELZI
        };
        let mut link = FakeLink::new(bootloader(flash, signature, sync_failures));
        super::flash(&mut link, &board, image)
    }
}
