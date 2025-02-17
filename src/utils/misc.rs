pub fn time_to_string(time: chrono::Duration) -> String {
    match time.num_seconds() {
        0..=59 => {
            let second_suffix = if time.num_seconds() == 1 { "s" } else { "" };
            format!("{} second{}", time.num_seconds(), second_suffix)
        }
        60..=3599 => {
            let minute_suffix = if time.num_minutes() == 1 { "s" } else { "" };
            format!("{} minute{}", time.num_minutes(), minute_suffix)
        }
        3600..=86399 => {
            let hour_suffix = if time.num_hours() == 1 { "s" } else { "" };
            format!("{} hour{}", time.num_hours(), hour_suffix)
        }
        _ => {
            let day_suffix = if time.num_days() == 1 { "s" } else { "" };
            format!("{} day{}", time.num_days(), day_suffix)
        }
    }
}
