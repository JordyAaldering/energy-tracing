mod rapl;

use std::{fs::File, io::{BufWriter, Write}, path::Path, sync::{LazyLock, Mutex, OnceLock}, time::Instant};

use crate::rapl::RaplReader;

static START: OnceLock<Instant> =
    OnceLock::new();

static RAPL: LazyLock<RaplReader> =
    LazyLock::new(|| RaplReader::new("/sys/class/powercap/intel-rapl:0").unwrap());

static TRACE_EVENTS: LazyLock<Mutex<Vec<TraceEvent>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[cfg(feature = "gap")]
pub static GAP_SNAPSHOTS: LazyLock<Mutex<HashMap<&'static str, Snapshot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
pub struct TraceEvent {
    pub region: &'static str,
    pub start_ns: u64,
    pub duration_ns: u64,
    pub energy_uj: u64,
}

#[derive(Clone, Copy)]
pub struct Snapshot {
    duration_ns: u64,
    energy_uj: u64,
}

impl Snapshot {
    pub fn now() -> Self {
        Self {
            duration_ns: Instant::now().duration_since(*START.get_or_init(Instant::now)).as_nanos() as u64,
            energy_uj: RAPL.read_energy_uj().unwrap(),
        }
    }
}

#[macro_export]
macro_rules! trace_region {
    ($region:literal, $block:block) => {{
        let start = $crate::Snapshot::now();
        $crate::record_gap($region, start);

        let result = { $block };

        let end = $crate::Snapshot::now();
        $crate::record_segment($region, start, end);
        $crate::remember_gap($region, end);

        result
    }};
}

#[allow(unused_variables)]
pub fn record_gap(region: &'static str, start: Snapshot) {
    #[cfg(feature = "gap")]
    if let Some(last) = GAP_SNAPSHOTS.lock().unwrap().get(region).copied() {
        record_segment(region, last, start);
    }
}

#[allow(unused_variables)]
pub fn remember_gap(region: &'static str, end: Snapshot) {
    #[cfg(feature = "gap")]
    GAP_SNAPSHOTS.lock().unwrap().insert(region, end);
}

pub fn record_segment(region: &'static str, start: Snapshot, end: Snapshot) {
    let duration_ns = end.duration_ns - start.duration_ns;
    let energy_uj = RAPL.delta_energy_uj(start.energy_uj, end.energy_uj);

    TRACE_EVENTS.lock().unwrap().push(TraceEvent {
        region,
        start_ns: start.duration_ns,
        duration_ns,
        energy_uj,
    });
}

pub fn write_traces<P>(path: P)
where
    P: AsRef<Path>,
{
    let traces = TRACE_EVENTS.lock().unwrap();

    let file = File::create(path).unwrap();
    let mut w = BufWriter::new(file);

    writeln!(w, "region,start_ns,duration_ns,energy_uj").unwrap();

    for trace in traces.iter() {
        writeln!(w, "{},{},{},{}", trace.region, trace.start_ns, trace.duration_ns, trace.energy_uj).unwrap();
    }
}
