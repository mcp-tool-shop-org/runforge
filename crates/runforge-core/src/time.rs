/// Order key for a backpropagate timestamp. `None` means the text is not a time,
/// and those rows sort last. A missing zone is ordered as UTC.
pub(crate) fn timestamp_ord(text: &str) -> Option<i128> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let (date, rest) = split_date(text)?;
    let (year, month, day) = parse_date(date)?;
    let (clock, offset_micros) = split_offset(rest)?;
    let (hour, minute, second, micros) = parse_clock(clock)?;
    let days = days_from_civil(year, month, day)?;
    Some(
        days as i128 * 86_400_000_000
            + hour as i128 * 3_600_000_000
            + minute as i128 * 60_000_000
            + second as i128 * 1_000_000
            + micros as i128
            - offset_micros,
    )
}

fn split_date(text: &str) -> Option<(&str, &str)> {
    text.split_once('T')
        .or_else(|| text.split_once('t'))
        .or_else(|| text.split_once(' '))
}

fn parse_date(date: &str) -> Option<(i32, u32, u32)> {
    let mut parts = date.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) {
        return None;
    }
    if day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap(year: i32) -> bool {
    let div = |n| year.rem_euclid(n) == 0;
    div(4) && (!div(100) || div(400))
}

fn parse_clock(clock: &str) -> Option<(u32, u32, u32, u32)> {
    let mut parts = clock.split(':');
    let hour: u32 = parts.next()?.parse().ok()?;
    let minute: u32 = parts.next()?.parse().ok()?;
    let seconds = parts.next()?;
    if parts.next().is_some() || hour > 23 || minute > 59 {
        return None;
    }
    let (second, micros) = parse_seconds(seconds)?;
    if second > 60 {
        return None;
    }
    Some((hour, minute, second, micros))
}

fn parse_seconds(text: &str) -> Option<(u32, u32)> {
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty() || !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let second: u32 = whole.parse().ok()?;
    let mut digits = frac.chars().take(6).collect::<String>();
    while digits.len() < 6 && !frac.is_empty() {
        digits.push('0');
    }
    let micros = if digits.is_empty() {
        0
    } else {
        digits.parse().ok()?
    };
    Some((second, micros))
}

/// `rest` is the clock plus an optional zone. The returned offset is microseconds
/// to subtract from the clock so the key is UTC.
fn split_offset(rest: &str) -> Option<(&str, i128)> {
    if rest.is_empty() {
        return None;
    }
    let last = rest.as_bytes()[rest.len() - 1];
    if last == b'Z' || last == b'z' {
        return Some((&rest[..rest.len() - 1], 0));
    }
    let Some(mark) = rest.rfind(['+', '-']) else {
        return Some((rest, 0));
    };
    if mark == 0 {
        return Some((rest, 0));
    }
    let clock = &rest[..mark];
    let zone = &rest[mark..];
    let sign: i128 = if zone.starts_with('+') { 1 } else { -1 };
    let body = &zone[1..];
    let (hour, minute) = if let Some((h, m)) = body.split_once(':') {
        (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?)
    } else if body.len() == 4 && body.chars().all(|c| c.is_ascii_digit()) {
        (body[..2].parse().ok()?, body[2..].parse().ok()?)
    } else {
        return Some((rest, 0));
    };
    if hour > 23 || minute > 59 {
        return None;
    }
    let offset = sign * (hour as i128 * 3_600_000_000 + minute as i128 * 60_000_000);
    Some((clock, offset))
}

/// Days since the Unix epoch. Invalid calendar days return `None`.
fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = (year - era * 400) as u64;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era as i64 * 146_097 + doe as i64 - 719_468)
}

#[cfg(test)]
mod tests {
    use super::timestamp_ord;

    #[test]
    fn newer_clock_sorts_after() {
        let early = timestamp_ord("2026-05-21T04:54:14.646724").unwrap();
        let later = timestamp_ord("2026-05-21T04:54:16.254808").unwrap();
        assert!(later > early);
    }

    #[test]
    fn zone_and_z_name_the_same_instant() {
        let z = timestamp_ord("2026-05-21T04:54:16Z").unwrap();
        let offset = timestamp_ord("2026-05-21T05:54:16+01:00").unwrap();
        assert_eq!(z, offset);
    }

    #[test]
    fn bad_text_is_not_a_time() {
        assert!(timestamp_ord("not-a-time").is_none());
        assert!(timestamp_ord("2026-02-31T00:00:00").is_none());
    }
}
