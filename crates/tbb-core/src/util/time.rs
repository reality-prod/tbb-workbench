use chrono::{DateTime, Utc};
use std::time::SystemTime;

pub fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub fn system_time_ms(t: SystemTime) -> i64 {
    DateTime::<Utc>::from(t).timestamp_millis()
}
