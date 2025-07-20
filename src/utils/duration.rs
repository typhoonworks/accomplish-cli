use crate::api::errors::ApiError;
use chrono::{Duration, Utc};
use regex::Regex;

/// Parses a duration string and returns the datetime that many units ago from now
///
/// Supports mixed duration formats like:
/// - "24h" -> 24 hours ago
/// - "3h30m" -> 3 hours and 30 minutes ago  
/// - "2d" -> 2 days ago
/// - "1w" -> 1 week ago
/// - "1d12h30m" -> 1 day, 12 hours, and 30 minutes ago
pub fn parse_since_duration(since: &str) -> Result<String, ApiError> {
    let regex = Regex::new(r"(\d+)([wdhm])")
        .map_err(|e| ApiError::InvalidInput(format!("Failed to compile duration regex: {}", e)))?;

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
                    "Unsupported duration unit: {}",
                    unit
                )))
            }
        };

        total_duration += duration;
    }

    if !found_match {
        return Err(ApiError::InvalidInput(
            "Invalid duration format. Use combinations like '24h', '3h30m', '2d', '1w'".to_string(),
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
}
