
const SIZE_UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
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
    let seconds = megabytes_remaining / speed_mb_s;
    format_time(seconds)
}

trait PushU64 {
    fn push_u64(&mut self, number: u64);
}

impl PushU64 for String {
    fn push_u64(&mut self, mut number: u64) {
        if number == 0 {
            self.push('0');
            return;
        }

        let mut buffer = [0u8; 32];
        let mut offset = 0;

        while number > 0 {
            let digit = (number % 10) as u8;
            number /= 10;

            buffer[offset] = digit;
            offset += 1;
        }

        while offset != 0  {
            offset -= 1;
            let digit = char::from(buffer[offset] + b'0');
            self.push(digit);
        }
    }
}

const DAY: f64    = 60.0 * 60.0 * 24.0;
const HOUR: f64   = 60.0 * 60.0;
const MINUTE: f64 = 60.0;

pub fn format_time(seconds: f64) -> String {
    if seconds < 1.0 {
        return format!("{seconds:.1}s");
    }

    let mut time = seconds;
    let mut output = String::with_capacity(32);

    if time >= DAY {
        let days = (time / DAY) as u64;
        time -= days as f64 * DAY;
        output.push_u64(days);
        output.push_str("d ");
    }

    if time >= HOUR {
        let hours = (time / HOUR) as u64;
        time -= hours as f64 * HOUR;
        output.push_u64(hours);
        output.push_str("h ");
    }

    if time >= MINUTE {
        let minutes = (time / MINUTE) as u64;
        time -= minutes as f64 * MINUTE;
        output.push_u64(minutes);
        output.push_str("m ");
    }

    if time >= 1.0 {
        let seconds = time as u64;
        output.push_u64(seconds);
        output.push('s');
    }

    return output
}
