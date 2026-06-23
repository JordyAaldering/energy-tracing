use std::{thread::sleep, time::{Duration, Instant}};

use energy_tracing::*;

fn main() {
    const DUR: Duration = Duration::from_millis(100);

    for _ in 0..20 {
        let previous = trace_start();
        let instant = Instant::now();

        sleep(DUR);

        let duration = instant.elapsed();
        let energy_uj = trace_stop(previous);
        println!("{:.2}W", (energy_uj as f32 / 1_000_000.0) / duration.as_secs_f32());
    }
}
