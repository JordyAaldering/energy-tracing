mod rapl;

use std::{collections::HashMap, sync::{LazyLock, Mutex, OnceLock}, time::Instant};

use crate::rapl::RaplReader;

static TIME: OnceLock<Instant> = OnceLock::new();

static RAPL: LazyLock<RaplReader> =
    LazyLock::new(|| RaplReader::new("/sys/class/powercap/intel-rapl:0").unwrap());

static TRACE_EVENTS: LazyLock<Mutex<Vec<TraceEvent>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[macro_export]
macro_rules! trace_region {
    ($name:literal, $block:block) => {{
        let start_ns = $crate::now_ns();
        let energy_start = $crate::trace_start();

        let result = { $block };

        let end_ns = $crate::now_ns();
        let duration_ns = end_ns - start_ns;
        let energy_uj = $crate::trace_stop(energy_start);

        $crate::record_event($crate::TraceEvent {
            name: $name,
            start_ns,
            duration_ns,
            energy_uj,
        });

        result
    }};
}

fn program_start() -> &'static Instant {
    TIME.get_or_init(Instant::now)
}

pub fn now_ns() -> u64 {
    let start = program_start();
    start.elapsed().as_nanos() as u64
}

pub fn trace_start() -> u64 {
    RAPL.read_energy_uj().unwrap()
}

pub fn trace_stop(previous: u64) -> u64 {
    let current = RAPL.read_energy_uj().unwrap();
    RAPL.delta_energy_uj(previous, current)
}

#[derive(Clone)]
pub struct TraceEvent {
    pub name: &'static str,
    pub start_ns: u64,
    pub duration_ns: u64,
    pub energy_uj: u64,
}

pub fn record_event(event: TraceEvent) {
    let mut log = TRACE_EVENTS.lock().unwrap();
    log.push(event);
}

pub fn print_trace_events() {
    let traces = TRACE_EVENTS.lock().unwrap();

    for trace in traces.iter() {
        println!("{:<10}: {:.3}ms {:.3}mJ", trace.name, trace.duration_ns as f64 / 1000.0, trace.energy_uj as f64 * 1000.0)
    }
}

pub fn print_trace_report() {
    let events = TRACE_EVENTS.lock().unwrap();

    let mut map: HashMap<&'static str, (u64, u64, u64)> = HashMap::new();

    for e in events.iter() {
        let entry = map
            .entry(e.name)
            .or_insert((0, 0, 0));

        entry.0 += 1;
        entry.1 += e.duration_ns;
        entry.2 += e.energy_uj;
    }

    let mut events = map.into_iter().collect::<Vec<_>>();
    events.sort_unstable_by_key(|e| e.0);

    println!(
        "{:<20} {:>10} {:>15} {:>15} {:>15} {:>15}",
        "Region", "Calls", "Time (ms)", "Avg. Time", "Energy (mJ)", "Avg. Energy"
    );

    for (region, (calls, duration_ns, energy_uj)) in events {
        println!(
            "{:<20} {:>10} {:>15.3} {:>15.3} {:>15.3} {:>15.3}",
            region,
            calls,
            duration_ns as f64 / 1_000_000.0,
            (duration_ns as f64 / 1_000_000.0) / calls as f64,
            energy_uj as f64 / 1_000.0,
            (energy_uj as f64 / 1_000.0) / calls as f64,
        );
    }
}
