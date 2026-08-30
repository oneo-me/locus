pub const LIBRARY_REFERENCE_SCHEME: &str = "locus://";

pub fn library_reference_path(reference: &str) -> Option<&str> {
    let path = reference.strip_prefix(LIBRARY_REFERENCE_SCHEME)?;
    if path.is_empty()
        || path.contains(['\\', '\0'])
        || path
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return None;
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_library_references() {
        assert_eq!(
            library_reference_path("locus://projects/locus/architecture"),
            Some("projects/locus/architecture")
        );
    }

    #[test]
    fn rejects_noncanonical_library_references() {
        for reference in [
            "https://example.com",
            "locus://",
            "locus:///projects/locus",
            "locus://projects//locus",
            "locus://projects/../locus",
            "locus://projects\\locus",
        ] {
            assert_eq!(library_reference_path(reference), None);
        }
    }
}
