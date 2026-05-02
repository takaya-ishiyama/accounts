use crate::{domain::clock::Clock, infrastructure::time::now_ms};

pub struct WorkerClock;

impl Clock for WorkerClock {
    fn now_ms(&self) -> i64 {
        now_ms()
    }
}
