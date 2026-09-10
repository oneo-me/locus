pub const LIBRARY_REFERENCE_SCHEME: &str = "locus://";

pub fn library_reference_path(reference: &str) -> Option<String> {
    let path = reference.strip_prefix(LIBRARY_REFERENCE_SCHEME)?;
    if path.is_empty() || path.contains(['\\', '\0', '?', '#']) {
        return None;
    }
    let mut segments = Vec::new();
    for segment in path.split('/') {
        let segment = percent_encoding::percent_decode_str(segment)
            .decode_utf8()
            .ok()?;
        if segment.is_empty()
            || matches!(segment.as_ref(), "." | "..")
            || segment.contains(['/', '\\', '\0'])
        {
            return None;
        }
        segments.push(segment.into_owned());
    }
    Some(segments.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_library_references() {
        assert_eq!(
            library_reference_path("locus://projects/locus/architecture").as_deref(),
            Some("projects/locus/architecture")
        );
        assert_eq!(
            library_reference_path("locus://旅行%20计划/100%25").as_deref(),
            Some("旅行 计划/100%")
        );
        assert_eq!(
            library_reference_path("locus://literal%252e").as_deref(),
            Some("literal%2e")
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
            "locus://note/%2e%2e",
            "locus://note%2Fother",
            "locus://note%5Cother",
            "locus://note%00",
            "locus://%FF",
            "locus://note?query",
            "locus://note#fragment",
        ] {
            assert_eq!(library_reference_path(reference), None, "{reference}");
        }
    }
}
