use std::time::Duration;

use bytesize::ByteSize;
use humantime::format_duration;
use ms_converter;

// -- This will pretty-print an ETA from milliseconds.
pub fn ms_to_eta_string(ms: &u64) -> String {
    let eta = format_duration(Duration::from_millis(ms.clone())).to_string();

    if format!("{eta}") == "0s" {
        String::from("Infinite")
    } else {
        eta
    }
}

// -- Converts human-readable string (from radarr or sonarr API) to milliseconds.
pub fn string_to_ms(string: &String) -> Result<i64, ms_converter::Error> {
    ms_converter::ms(string)
}

// -- This will convert for example "1 TB", "512 MB", <"1.5 GB" to 1500000 (bytes)>.
pub fn string_bytesize_to_bytes(string: &String) -> u64 {
    match string.parse::<ByteSize>() {
        Ok(size) => size.as_u64(),
        Err(_) => 0,
    }
}

// -- Converts human-readable string (from radarr or sonarr API) to milliseconds.
pub fn string_hms_to_ms(string: &String) -> u64 {
    let parts: Vec<&str> = string.split(|c| c == ':' || c == '.').collect();

    // Check if we have at least hours, minutes, and seconds
    if parts.len() < 3 {
        return 0;
    }

    let mut days: u64 = 0;
    let hours: u64;
    let minutes: u64;
    let seconds: u64;

    match parts.len() {
        // For the format "12:34:56"
        3 => {
            hours = parts[0].parse().unwrap_or_else(|_| 0);
            minutes = parts[1].parse().unwrap_or_else(|_| 0);
            seconds = parts[2].parse().unwrap_or_else(|_| 0);
        }
        // For the format "12.34:56:78"
        4 => {
            days = parts[0].parse().unwrap_or_else(|_| 0);
            hours = parts[1].parse().unwrap_or_else(|_| 0);
            minutes = parts[2].parse().unwrap_or_else(|_| 0);
            seconds = parts[3].parse().unwrap_or_else(|_| 0);
        }
        _ => return 0,
    }

    // Calculate total milliseconds
    let total_ms = ((days * 24 + hours) * 3600 + minutes * 60 + seconds) * 1000;
    total_ms
}