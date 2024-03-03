
const SIZE_UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
const TIME_UNITS: [&str; 5] = ["s", "min", "h", "d", "y"];
const MB_1: f64 = 1048576.0;

pub fn format_size(file_size: u64) -> String {
    let mut value = file_size as f64;
    let mut unit_index = 0;
    while value > 1024.0 {
        value /= 1024.0;
        unit_index += 1;
    }
    let unit = SIZE_UNITS[unit_index];
    return format!("{value:.2}{unit}");
}

pub fn format_eta(bytes_progress: u64, all_bytes: u64, speed_mb_s: f64) -> String {
    let megabytes_remaining = (all_bytes - bytes_progress) as f64 / MB_1;
    let seconds = (megabytes_remaining / speed_mb_s);
    format_time(seconds)
}

pub fn format_time(seconds: f64) -> String {
    let mut time = seconds;
    let mut unit_index = 0;
    while unit_index < 2 && time >= 60.0 {
        time /= 60.0;
        unit_index += 1;
    }

    // formatting above 72 hours
    if time < 72.0 {
        let unit = TIME_UNITS[unit_index];
        return format!("{time:.1}{unit}");
    }
    time /= 24.0;
    unit_index += 1;

    let unit = TIME_UNITS[unit_index];
    return format!("{time:.1}{unit}");
}