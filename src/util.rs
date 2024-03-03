use std::time::Duration;

const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
const MB_1: f64 = 1048576.0;

pub fn format_size(file_size: u64) -> String {
    let mut value = file_size as f64;
    let mut unit_index = 0;
    while value > 1024.0 {
        value /= 1024.0;
        unit_index += 1;
    }
    let unit = UNITS[unit_index];
    return format!("{value:.2}{unit}");
}

pub fn eta(bytes_progress: u64, all_bytes: u64, speed_mb_s: f64) -> Duration {
    let megabytes_remaining = (all_bytes - bytes_progress) as f64 / MB_1;
    let millis = (megabytes_remaining / speed_mb_s) * 1000.0;
    Duration::from_millis(millis as u64)
}