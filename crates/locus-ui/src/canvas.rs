pub struct CanvasMetadata<'a> {
    title: &'a str,
    description: &'a str,
}

impl<'a> CanvasMetadata<'a> {
    pub fn parse(header: &'a str) -> Option<Self> {
        let (title, description) = header.split_once('|')?;
        let title = title.trim();
        let description = description.trim();
        if title.is_empty() || description.is_empty() {
            return None;
        }
        Some(Self { title, description })
    }

    pub fn title(&self) -> &'a str {
        self.title
    }

    pub fn description(&self) -> &'a str {
        self.description
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_author_supplied_title_and_description() {
        for header in ["", "Title", "Title |", "| Description"] {
            assert!(CanvasMetadata::parse(header).is_none());
        }
        let metadata = CanvasMetadata::parse(" 暖暮样板 | 色彩、字体与交互组件 ").unwrap();
        assert_eq!(metadata.title(), "暖暮样板");
        assert_eq!(metadata.description(), "色彩、字体与交互组件");
    }
}
