use std::{fs::{self, File}, io, os::unix::fs::FileExt, path::Path};

pub struct RaplReader {
    energy_file: File,
    max_energy_range_uj: u64,
}

impl RaplReader {
    pub fn new<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let energy_path = path.as_ref().join("energy_uj");
        let energy_file = File::open(energy_path)?;

        let max_energy_range_path = path.as_ref().join("max_energy_range_uj");
        let max_energy_range_uj = fs::read_to_string(max_energy_range_path)?
            .trim()
            .parse::<u64>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(Self {
            energy_file,
            max_energy_range_uj,
        })
    }

    pub fn read_energy_uj(&self) -> io::Result<u64> {
        let mut buf = [0u8; 32];
        let n = self.energy_file.read_at(&mut buf, 0)?;

        let mut energy_uj = 0u64;
        for &b in &buf[..n] {
            if b == b'\n' {
                break;
            }
            energy_uj = energy_uj * 10 + (b - b'0') as u64;
        }

        Ok(energy_uj)
    }

    pub fn delta_energy_uj(&self, previous: u64, current: u64) -> u64 {
        if current >= previous {
            current - previous
        } else {
            current + (self.max_energy_range_uj - previous)
        }
    }
}
