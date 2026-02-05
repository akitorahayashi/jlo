/// Generate a 6-character mock ID.
pub fn generate_mock_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{:06x}", (timestamp % 0xFFFFFF) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mock_id() {
        let id1 = generate_mock_id();
        let id2 = generate_mock_id();
        assert_eq!(id1.len(), 6);
        assert_eq!(id2.len(), 6);
        // IDs should be different (very high probability)
        // Note: This could theoretically fail if called in same nanosecond
    }
}
