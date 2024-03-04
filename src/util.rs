
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

const DAY: f64    = 60.0 * 60.0 * 24.0;
const HOUR: f64   = 60.0 * 60.0;
const MINUTE: f64 = 60.0;

pub fn format_time(seconds: f64) -> String {
    let mut time = seconds;
    let mut output = String::new();

    if time >= DAY {
        let days = (time / DAY) as u64;
        time -= days as f64 * DAY;
        output = format!("{days}d");
    }

    if time >= HOUR {
        let hours = (time / HOUR) as u64;
        time -= hours as f64 * HOUR;
        output = format!("{output} {hours}h");
    }

    if time >= MINUTE {
        let minutes = (time / MINUTE) as u64;
        time -= minutes as f64 * MINUTE;
        output = format!("{output} {minutes}m");
    }

    if time >= 1.0 {
        let seconds = time as u64;
        output = format!("{output} {seconds}s");
    }

    return output
}

pub fn format_time_old(seconds: f64) -> String {
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
