use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const SAMPLE: u64 = 64 * 1024;

struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

pub fn compute(path: &Path, size: u64) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("no se pudo abrir {}", path.display()))?;
    let mut hasher = Fnv::new();
    hasher.update(&size.to_le_bytes());

    let mut buffer = vec![0u8; size.min(SAMPLE) as usize];
    let first = file.read(&mut buffer)?;
    hasher.update(&buffer[..first]);

    if size > SAMPLE {
        let start = size.saturating_sub(SAMPLE);
        file.seek(SeekFrom::Start(start))?;
        let mut tail = vec![0u8; SAMPLE as usize];
        let last = file.read(&mut tail)?;
        hasher.update(&tail[..last]);
    }

    Ok(hasher.hex())
}
