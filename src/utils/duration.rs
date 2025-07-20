use crate::api::errors::ApiError;
use chrono::{Datelike, Duration, Local, TimeZone, Utc};
use regex::Regex;

/// Parses a duration string and returns the datetime that many units ago from now
///
/// Supports mixed duration formats like:
/// - "24h" -> 24 hours ago
/// - "3h30m" -> 3 hours and 30 minutes ago  
/// - "2d" -> 2 days ago
/// - "1w" -> 1 week ago
/// - "1d12h30m" -> 1 day, 12 hours, and 30 minutes ago
///
/// Also supports named expressions:
/// - "yesterday" -> From 00:00 yesterday up until now
/// - "today" -> From 00:00 today up until now
/// - "this-week" -> From 00:00 of the first day of the current week up until now
/// - "last-week" -> From 00:00 of the first day of the previous week through 23:59 of its last day
/// - "this-month" -> From 00:00 on the first day of the current month up until now
/// - "last-month" -> From 00:00 on the first day of the previous calendar month through its end
pub fn parse_since_duration(since: &str) -> Result<String, ApiError> {
    // Handle named expressions first
    match since {
        "yesterday" => {
            let now = Local::now();
            let yesterday = now - Duration::days(1);
            let start_of_yesterday = yesterday.date_naive().and_hms_opt(0, 0, 0).unwrap();
            let utc_start = Utc.from_local_datetime(&start_of_yesterday).unwrap();
            return Ok(utc_start.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
        "today" => {
            let now = Local::now();
            let start_of_today = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
            let utc_start = Utc.from_local_datetime(&start_of_today).unwrap();
            return Ok(utc_start.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
        "this-week" => {
            let now = Local::now();
            let days_since_monday = now.weekday().num_days_from_monday();
            let monday = now - Duration::days(days_since_monday as i64);
            let start_of_week = monday.date_naive().and_hms_opt(0, 0, 0).unwrap();
            let utc_start = Utc.from_local_datetime(&start_of_week).unwrap();
            return Ok(utc_start.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
        "last-week" => {
            let now = Local::now();
            let days_since_monday = now.weekday().num_days_from_monday();
            let this_monday = now - Duration::days(days_since_monday as i64);
            let last_monday = this_monday - Duration::days(7);
            let start_of_last_week = last_monday.date_naive().and_hms_opt(0, 0, 0).unwrap();
            let utc_start = Utc.from_local_datetime(&start_of_last_week).unwrap();
            return Ok(utc_start.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
        "this-month" => {
            let now = Local::now();
            let start_of_month = now
                .date_naive()
                .with_day(1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            let utc_start = Utc.from_local_datetime(&start_of_month).unwrap();
            return Ok(utc_start.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
        "last-month" => {
            let now = Local::now();
            let first_of_this_month = now.date_naive().with_day(1).unwrap();
            let last_month = if now.month() == 1 {
                first_of_this_month
                    .with_year(now.year() - 1)
                    .unwrap()
                    .with_month(12)
                    .unwrap()
            } else {
                first_of_this_month.with_month(now.month() - 1).unwrap()
            };
            let start_of_last_month = last_month.and_hms_opt(0, 0, 0).unwrap();
            let utc_start = Utc.from_local_datetime(&start_of_last_month).unwrap();
            return Ok(utc_start.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
        _ => {
            // Continue with regex parsing for duration expressions
        }
    }

    let regex = Regex::new(r"(\d+)([wdhm])")
        .map_err(|e| ApiError::InvalidInput(format!("Failed to compile duration regex: {e}")))?;

    let mut total_duration = Duration::zero();
    let mut found_match = false;

    for cap in regex.captures_iter(since) {
        found_match = true;
        let value: i64 = cap[1].parse().map_err(|_| {
            ApiError::InvalidInput(format!("Invalid number in duration: {}", &cap[1]))
        })?;

        let unit = &cap[2];
        let duration = match unit {
            "w" => Duration::weeks(value),
            "d" => Duration::days(value),
            "h" => Duration::hours(value),
            "m" => Duration::minutes(value),
            _ => {
                return Err(ApiError::InvalidInput(format!(
                    "Unsupported duration unit: {unit}"
                )))
            }
        };

        total_duration += duration;
    }

    if !found_match {
        return Err(ApiError::InvalidInput(
            "Invalid duration format. Use combinations like '24h', '3h30m', '2d', '1w' or named expressions: 'yesterday', 'today', 'this-week', 'last-week', 'this-month', 'last-month'".to_string(),
        ));
    }

    let now = Utc::now();
    let from_time = now - total_duration;

    Ok(from_time.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    #[test]
    fn test_parse_hours() {
        let result = parse_since_duration("24h").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();
        let expected = Utc::now() - Duration::hours(24);

        // Allow for small differences in execution time
        let diff = (parsed - expected).abs();
        assert!(diff < Duration::seconds(1));
    }

    #[test]
    fn test_parse_mixed_duration() {
        let result = parse_since_duration("1d12h30m").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();
        let expected = Utc::now() - Duration::days(1) - Duration::hours(12) - Duration::minutes(30);

        let diff = (parsed - expected).abs();
        assert!(diff < Duration::seconds(1));
    }

    #[test]
    fn test_parse_days() {
        let result = parse_since_duration("7d").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();
        let expected = Utc::now() - Duration::days(7);

        let diff = (parsed - expected).abs();
        assert!(diff < Duration::seconds(1));
    }

    #[test]
    fn test_parse_weeks() {
        let result = parse_since_duration("2w").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();
        let expected = Utc::now() - Duration::weeks(2);

        let diff = (parsed - expected).abs();
        assert!(diff < Duration::seconds(1));
    }

    #[test]
    fn test_parse_minutes() {
        let result = parse_since_duration("90m").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();
        let expected = Utc::now() - Duration::minutes(90);

        let diff = (parsed - expected).abs();
        assert!(diff < Duration::seconds(1));
    }

    #[test]
    fn test_invalid_format() {
        let result = parse_since_duration("invalid");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid duration format"));
    }

    #[test]
    fn test_unsupported_unit() {
        let result = parse_since_duration("5s");
        assert!(result.is_err());
        // The regex doesn't match 's' so it should return "Invalid duration format" instead
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid duration format"));
    }

    #[test]
    fn test_empty_string() {
        let result = parse_since_duration("");
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_mixed() {
        let result = parse_since_duration("2w3d4h15m").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();
        let expected = Utc::now()
            - Duration::weeks(2)
            - Duration::days(3)
            - Duration::hours(4)
            - Duration::minutes(15);

        let diff = (parsed - expected).abs();
        assert!(diff < Duration::seconds(1));
    }

    #[test]
    fn test_yesterday() {
        let result = parse_since_duration("yesterday").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();

        let now = Local::now();
        let yesterday = now - Duration::days(1);
        let expected_start = yesterday.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let expected_utc = Utc.from_local_datetime(&expected_start).unwrap();

        // Should be exactly the start of yesterday
        assert_eq!(parsed, expected_utc);
    }

    #[test]
    fn test_today() {
        let result = parse_since_duration("today").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();

        let now = Local::now();
        let expected_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let expected_utc = Utc.from_local_datetime(&expected_start).unwrap();

        // Should be exactly the start of today
        assert_eq!(parsed, expected_utc);
    }

    #[test]
    fn test_this_week() {
        let result = parse_since_duration("this-week").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();

        let now = Local::now();
        let days_since_monday = now.weekday().num_days_from_monday();
        let monday = now - Duration::days(days_since_monday as i64);
        let expected_start = monday.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let expected_utc = Utc.from_local_datetime(&expected_start).unwrap();

        assert_eq!(parsed, expected_utc);
    }

    #[test]
    fn test_last_week() {
        let result = parse_since_duration("last-week").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();

        let now = Local::now();
        let days_since_monday = now.weekday().num_days_from_monday();
        let this_monday = now - Duration::days(days_since_monday as i64);
        let last_monday = this_monday - Duration::days(7);
        let expected_start = last_monday.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let expected_utc = Utc.from_local_datetime(&expected_start).unwrap();

        assert_eq!(parsed, expected_utc);
    }

    #[test]
    fn test_this_month() {
        let result = parse_since_duration("this-month").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();

        let now = Local::now();
        let expected_start = now
            .date_naive()
            .with_day(1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let expected_utc = Utc.from_local_datetime(&expected_start).unwrap();

        assert_eq!(parsed, expected_utc);
    }

    #[test]
    fn test_last_month() {
        let result = parse_since_duration("last-month").unwrap();
        let parsed: DateTime<Utc> = result.parse().unwrap();

        let now = Local::now();
        let first_of_this_month = now.date_naive().with_day(1).unwrap();
        let last_month = if now.month() == 1 {
            first_of_this_month
                .with_year(now.year() - 1)
                .unwrap()
                .with_month(12)
                .unwrap()
        } else {
            first_of_this_month.with_month(now.month() - 1).unwrap()
        };
        let expected_start = last_month.and_hms_opt(0, 0, 0).unwrap();
        let expected_utc = Utc.from_local_datetime(&expected_start).unwrap();

        assert_eq!(parsed, expected_utc);
    }

    #[test]
    fn test_invalid_named_expression() {
        let result = parse_since_duration("invalid-expression");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid duration format"));
    }
}
