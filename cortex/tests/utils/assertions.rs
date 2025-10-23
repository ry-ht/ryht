//! Custom Assertion Helpers for Complex Data Structures

use anyhow::Result;

/// Assert that a string contains all of the given substrings
pub fn assert_contains_all(text: &str, needles: &[&str]) -> Result<()> {
    for needle in needles {
        if !text.contains(needle) {
            anyhow::bail!("Text does not contain expected substring: '{}'", needle);
        }
    }
    Ok(())
}

/// Assert that a string contains any of the given substrings
pub fn assert_contains_any(text: &str, needles: &[&str]) -> Result<()> {
    for needle in needles {
        if text.contains(needle) {
            return Ok(());
        }
    }
    anyhow::bail!("Text does not contain any of the expected substrings: {:?}", needles);
}

/// Assert that a value is within a percentage range of expected
pub fn assert_within_percent(actual: f64, expected: f64, percent: f64) -> Result<()> {
    let diff = (actual - expected).abs();
    let allowed = expected * (percent / 100.0);

    if diff > allowed {
        anyhow::bail!(
            "Value {} is not within {}% of expected {} (diff: {})",
            actual,
            percent,
            expected,
            diff
        );
    }
    Ok(())
}

/// Assert that a duration is within a time range
pub fn assert_within_duration(
    actual: std::time::Duration,
    min: std::time::Duration,
    max: std::time::Duration,
) -> Result<()> {
    if actual < min {
        anyhow::bail!("Duration {:?} is less than minimum {:?}", actual, min);
    }
    if actual > max {
        anyhow::bail!("Duration {:?} exceeds maximum {:?}", actual, max);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_contains_all() {
        let text = "Hello world, this is a test";
        assert!(assert_contains_all(text, &["Hello", "world", "test"]).is_ok());
        assert!(assert_contains_all(text, &["missing"]).is_err());
    }

    #[test]
    fn test_assert_within_percent() {
        assert!(assert_within_percent(100.0, 95.0, 10.0).is_ok());
        assert!(assert_within_percent(100.0, 200.0, 10.0).is_err());
    }
}
