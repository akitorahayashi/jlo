use std::cmp::Ordering;

/// A simple version struct for parsing and comparing version strings (e.g. "1.2.3").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    parts: Vec<u32>,
}

impl Version {
    /// Parse a version string into a `Version` object.
    ///
    /// Returns `None` if the string contains non-numeric segments.
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<_> = s.split('.').map(|segment| segment.parse::<u32>()).collect();
        if parts.iter().any(|part| part.is_err()) {
            return None;
        }
        Some(Self { parts: parts.into_iter().map(|part| part.unwrap()).collect() })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let max_len = self.parts.len().max(other.parts.len());
        for idx in 0..max_len {
            let left_value = *self.parts.get(idx).unwrap_or(&0);
            let right_value = *other.parts.get(idx).unwrap_or(&0);
            match left_value.cmp(&right_value) {
                Ordering::Less => return Ordering::Less,
                Ordering::Greater => return Ordering::Greater,
                Ordering::Equal => {}
            }
        }
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert_eq!(Version::parse("1.2.3"), Some(Version { parts: vec![1, 2, 3] }));
        assert_eq!(Version::parse("1.0"), Some(Version { parts: vec![1, 0] }));
        assert_eq!(Version::parse("10.20.30"), Some(Version { parts: vec![10, 20, 30] }));
        assert_eq!(Version::parse("invalid"), None);
        assert_eq!(Version::parse("1.a.2"), None);
    }

    #[test]
    fn test_compare() {
        // Equal
        assert_eq!(
            Version::parse("1.2.3").unwrap().cmp(&Version::parse("1.2.3").unwrap()),
            Ordering::Equal
        );
        // Left greater
        assert!(Version::parse("1.2.4").unwrap() > Version::parse("1.2.3").unwrap());
        assert!(Version::parse("1.3.0").unwrap() > Version::parse("1.2.3").unwrap());
        assert!(Version::parse("2.0.0").unwrap() > Version::parse("1.2.3").unwrap());
        assert!(Version::parse("1.2.3.1").unwrap() > Version::parse("1.2.3").unwrap());
        // Left smaller
        assert!(Version::parse("1.2.2").unwrap() < Version::parse("1.2.3").unwrap());
        assert!(Version::parse("1.1.9").unwrap() < Version::parse("1.2.3").unwrap());
        assert!(Version::parse("0.9.9").unwrap() < Version::parse("1.2.3").unwrap());
        assert!(Version::parse("1.2").unwrap() < Version::parse("1.2.3").unwrap());
    }
}
