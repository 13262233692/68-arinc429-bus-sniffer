use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[cfg(target_family = "unix")]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(target_family = "windows")]
use std::os::windows::io::{AsRawHandle, RawHandle};

use anyhow::{anyhow, Context, Result};
use byteorder::{ByteOrder, LittleEndian};

use crate::core::word::ArincWord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionStrategy {
    RoundRobin,
    HashByLabel,
    FixedPort(u8),
    AllPortsBroadcast,
}

impl Default for InjectionStrategy {
    fn default() -> Self {
        InjectionStrategy::RoundRobin
    }
}

#[derive(Debug)]
pub struct HardwarePort {
    pub id: u8,
    #[cfg(target_family = "unix")]
    fd: Option<RawFd>,
    #[cfg(target_family = "windows")]
    handle: Option<RawHandle>,
    path: PathBuf,
    backend: PortBackend,
    file: Option<File>,
}

impl Clone for HardwarePort {
    fn clone(&self) -> Self {
        HardwarePort {
            id: self.id,
            #[cfg(target_family = "unix")]
            fd: self.fd,
            #[cfg(target_family = "windows")]
            handle: self.handle,
            path: self.path.clone(),
            backend: self.backend,
            file: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortBackend {
    DevCharDevice,
    SysfsUio,
    RawMemoryMap,
    UserSpaceTun,
    FileSink,
    Simulated,
}

impl HardwarePort {
    pub fn open(id: u8) -> Result<Self> {
        let dev_path = PathBuf::from(format!("/dev/arinc429/tx{}", id));
        let sysfs_path = PathBuf::from(format!("/sys/class/arinc429/port{}/tx", id));

        let (backend, file, fd) = if cfg!(target_family = "unix") && dev_path.exists() {
            let f = OpenOptions::new()
                .write(true)
                .read(false)
                .open(&dev_path)
                .with_context(|| format!("Cannot open ARINC device {}", dev_path.display()))?;
            #[cfg(target_family = "unix")]
            let fd = Some(f.as_raw_fd());
            #[cfg(target_family = "windows")]
            let fd = None;
            (PortBackend::DevCharDevice, Some(f), fd)
        } else if cfg!(target_family = "unix") && sysfs_path.exists() {
            let f = OpenOptions::new()
                .write(true)
                .open(&sysfs_path)
                .with_context(|| format!("Cannot open sysfs port {}", sysfs_path.display()))?;
            #[cfg(target_family = "unix")]
            let fd = Some(f.as_raw_fd());
            #[cfg(target_family = "windows")]
            let fd = None;
            (PortBackend::SysfsUio, Some(f), fd)
        } else {
            let sim_dir = if cfg!(target_family = "windows") {
                PathBuf::from(std::env::temp_dir()).join("arinc429-sim")
            } else {
                PathBuf::from("/tmp/arinc429-sim")
            };
            std::fs::create_dir_all(&sim_dir).ok();
            let sim_path = sim_dir.join(format!("tx{}.sock", id));
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&sim_path)
                .with_context(|| format!("Cannot open simulated port {}", sim_path.display()))?;
            #[cfg(target_family = "unix")]
            let fd = Some(f.as_raw_fd());
            #[cfg(target_family = "windows")]
            let fd = Some(f.as_raw_handle());
            (PortBackend::Simulated, Some(f), fd)
        };

        Ok(HardwarePort {
            id,
            #[cfg(target_family = "unix")]
            fd,
            #[cfg(target_family = "windows")]
            handle: fd,
            path: if backend == PortBackend::DevCharDevice { dev_path }
                else if backend == PortBackend::SysfsUio { sysfs_path }
                else {
                    if cfg!(target_family = "windows") {
                        PathBuf::from(std::env::temp_dir()).join("arinc429-sim").join(format!("tx{}.sock", id))
                    } else {
                        PathBuf::from(format!("/tmp/arinc429-sim/tx{}.sock", id))
                    }
                },
            backend,
            file,
        })
    }

    pub fn open_sink<P: AsRef<Path>>(id: u8, path: P) -> Result<Self> {
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_ref())?;
        #[cfg(target_family = "unix")]
        let fd = Some(f.as_raw_fd());
        #[cfg(target_family = "windows")]
        let handle = Some(f.as_raw_handle());
        Ok(HardwarePort {
            id,
            #[cfg(target_family = "unix")]
            fd,
            #[cfg(target_family = "windows")]
            handle,
            path: path.as_ref().to_path_buf(),
            backend: PortBackend::FileSink,
            file: Some(f),
        })
    }

    pub fn is_real_hardware(&self) -> bool {
        matches!(self.backend, PortBackend::DevCharDevice | PortBackend::SysfsUio | PortBackend::RawMemoryMap)
    }

    #[cfg(target_os = "linux")]
    unsafe fn syscall_write(&self, buf: &[u8]) -> io::Result<isize> {
        if let Some(fd) = self.fd {
            let res = libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len());
            if res < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(res)
            }
        } else {
            Err(io::Error::new(io::ErrorKind::NotConnected, "No file descriptor"))
        }
    }

    #[cfg(not(target_os = "linux"))]
    unsafe fn syscall_write(&self, buf: &[u8]) -> io::Result<isize> {
        self.fallback_write(buf)
    }

    fn fallback_write(&self, buf: &[u8]) -> io::Result<isize> {
        if let Some(mut f) = self.file.as_ref() {
            f.write_all(buf)?;
            Ok(buf.len() as isize)
        } else {
            Err(io::Error::new(io::ErrorKind::NotConnected, "Port not open"))
        }
    }

    pub fn write_raw_word(&self, raw: u32) -> Result<usize> {
        let mut buf = [0u8; 4];
        LittleEndian::write_u32(&mut buf, raw);

        let written = if matches!(self.backend, PortBackend::DevCharDevice) {
            #[cfg(target_os = "linux")]
            unsafe {
                self.syscall_write(&buf)? as usize
            }
            #[cfg(not(target_os = "linux"))]
            {
                self.fallback_write(&buf)? as usize
            }
        } else {
            let mut w_ts = Vec::with_capacity(12);
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros();
            w_ts.extend_from_slice(&ts.to_le_bytes());
            w_ts.extend_from_slice(&buf);
            match self.backend {
                PortBackend::FileSink | PortBackend::Simulated => {
                    if let Some(mut f) = self.file.as_ref() {
                        f.write_all(&w_ts)?;
                        w_ts.len()
                    } else {
                        0
                    }
                }
                _ => self.fallback_write(&buf)? as usize,
            }
        };

        if written == 0 {
            return Err(anyhow!("Write failed to port {}", self.id));
        }
        Ok(written)
    }
}

pub struct BusInjector {
    strategy: InjectionStrategy,
    stop_flag: Arc<AtomicBool>,
    total_injected: std::sync::atomic::AtomicU64,
    total_errors: std::sync::atomic::AtomicU64,
}

impl BusInjector {
    pub fn new(strategy: InjectionStrategy, stop_flag: Arc<AtomicBool>) -> Self {
        BusInjector {
            strategy,
            stop_flag,
            total_injected: Default::default(),
            total_errors: Default::default(),
        }
    }

    pub fn inject_word(
        &self,
        word: &ArincWord,
        port: Option<&HardwarePort>,
    ) -> Result<usize> {
        if self.stop_flag.load(Ordering::Relaxed) {
            return Ok(0);
        }
        let hw_port = port.ok_or_else(|| anyhow!("No available hardware port in pool"))?;
        match hw_port.write_raw_word(word.raw()) {
            Ok(n) => {
                self.total_injected.fetch_add(1, Ordering::Relaxed);
                Ok(n)
            }
            Err(e) => {
                self.total_errors.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    pub fn stats(&self) -> (u64, u64) {
        (self.total_injected.load(Ordering::Relaxed),
         self.total_errors.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::core::word::WordEndianness;

    #[test]
    fn test_inject_to_file_sink() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("tx_dump.bin");
        let port = HardwarePort::open_sink(7, &out).unwrap();
        assert_eq!(port.id, 7);
        let word = ArincWord::from_u32(0x12345678, WordEndianness::Standard);
        let n = port.write_raw_word(word.raw()).unwrap();
        assert!(n > 0);
        let md = std::fs::metadata(&out).unwrap();
        assert!(md.len() >= 4);
    }

    #[test]
    fn test_strategy_defaults() {
        let s = InjectionStrategy::default();
        assert_eq!(s, InjectionStrategy::RoundRobin);
    }
}
