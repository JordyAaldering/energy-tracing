use std::{env, fs, io, thread, time::Duration};

use energy_tracing::*;

fn main() {
    let frequency_hz: u64 = env::args().nth(1).map_or(1,
        |v| v.parse().unwrap());
    let num_samples: usize = env::args().nth(2).map_or(10,
        |v| v.parse().unwrap());

    let duration = Duration::from_millis(1000 / frequency_hz);

    trace_region!("total", {
        for _ in 0..num_samples {
            trace_region!("idle", {
                thread::sleep(duration);
            });
        }
    });

    write_traces(format!("target/idle_{}.csv", frequency_hz));
}
