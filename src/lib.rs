mod rapl;

use std::sync::LazyLock;

use crate::rapl::RaplReader;

static RAPL: LazyLock<RaplReader> = LazyLock::new(|| {
    RaplReader::new("/sys/class/powercap/intel-rapl:0").unwrap()
});

pub fn trace_start() -> u64 {
    RAPL.read_energy_uj().unwrap()
}

pub fn trace_stop(previous: u64) -> u64 {
    let current = RAPL.read_energy_uj().unwrap();
    RAPL.delta_energy_uj(previous, current)
}
