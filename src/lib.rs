mod rapl;

use std::{collections::HashMap, io, sync::{LazyLock, Mutex, OnceLock}, time::Instant};

use crate::rapl::RaplReader;

pub static START_TIME: OnceLock<Instant> =
    OnceLock::new();

pub static RAPL: LazyLock<RaplReader> =
    LazyLock::new(|| RaplReader::new("/sys/class/powercap/intel-rapl:0").unwrap());

pub static LAST_REGION_SNAPSHOTS: LazyLock<Mutex<HashMap<&'static str, Snapshot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub static TRACE_EVENTS: LazyLock<Mutex<Vec<TraceEvent>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

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
            duration_ns: Instant::now().duration_since(*START_TIME.get_or_init(Instant::now)).as_nanos() as u64,
            energy_uj: RAPL.read_energy_uj().unwrap(),
        }
    }
}

#[macro_export]
macro_rules! trace_region {
    ($region:literal, $block:block) => {{
        let start = $crate::Snapshot::now();

        // Record gap since previous execution of this region
        if let Some(previous_end) = {
            $crate::LAST_REGION_SNAPSHOTS
                .lock()
                .unwrap()
                .get($region)
                .copied()
        } {
            $crate::record_segment(concat!("gap_", $region), previous_end, start);
        }

        let result = { $block };

        let end = $crate::Snapshot::now();

        $crate::record_segment($region, start, end);

        // Remember when this region last finished
        // This assumes regions with the same name are never nested
        $crate::LAST_REGION_SNAPSHOTS
            .lock()
            .unwrap()
            .insert($region, end);

        result
    }};
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

pub fn print_trace_events<W>(w: &mut W)
where
    W: io::Write,
{
    let traces = TRACE_EVENTS.lock().unwrap();

    writeln!(w, "region,start_ns,duration_ns,energy_uj").unwrap();
    for trace in traces.iter() {
        writeln!(w, "{},{},{},{}", trace.region, trace.start_ns, trace.duration_ns, trace.energy_uj).unwrap();
    }
}

pub fn print_trace_report<W>(w: &mut W)
where
    W: io::Write,
{
    let events = TRACE_EVENTS.lock().unwrap();

    let mut map: HashMap<&'static str, (u64, u64, u64)> = HashMap::new();

    for e in events.iter() {
        let entry = map
            .entry(e.region)
            .or_insert((0, 0, 0));

        entry.0 += 1;
        entry.1 += e.duration_ns;
        entry.2 += e.energy_uj;
    }

    let mut events = map.into_iter().collect::<Vec<_>>();
    events.sort_unstable_by_key(|e| e.0);

    writeln!(
        w,
        "{:<20} {:>10} {:>15} {:>15} {:>15} {:>15}",
        "Region", "Calls", "Time (ms)", "Avg. Time", "Energy (mJ)", "Avg. Energy"
    ).unwrap();

    for (region, (calls, duration_ns, energy_uj)) in events {
        writeln!(
            w,
            "{:<20} {:>10} {:>15.3} {:>15.3} {:>15.3} {:>15.3}",
            region,
            calls,
            duration_ns as f64 / 1_000_000.0,
            (duration_ns as f64 / 1_000_000.0) / calls as f64,
            energy_uj as f64 / 1_000.0,
            (energy_uj as f64 / 1_000.0) / calls as f64,
        ).unwrap();
    }
}
