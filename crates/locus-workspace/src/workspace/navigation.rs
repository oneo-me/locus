#[derive(Debug, Default)]
pub(super) struct LibraryNavigation {
    back: Vec<String>,
    forward: Vec<String>,
}

impl LibraryNavigation {
    pub(super) fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub(super) fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub(super) fn visit(&mut self, current: Option<&str>, next: &str) {
        if current == Some(next) {
            return;
        }
        if let Some(current) = current {
            self.back.push(current.to_owned());
        }
        self.forward.clear();
    }

    pub(super) fn go_back(&mut self, current: Option<&str>) -> Option<String> {
        let target = self.back.pop()?;
        if let Some(current) = current.filter(|current| *current != target) {
            self.forward.push(current.to_owned());
        }
        Some(target)
    }

    pub(super) fn go_forward(&mut self, current: Option<&str>) -> Option<String> {
        let target = self.forward.pop()?;
        if let Some(current) = current.filter(|current| *current != target) {
            self.back.push(current.to_owned());
        }
        Some(target)
    }

    pub(super) fn retain(&mut self, mut keep: impl FnMut(&str) -> bool) {
        self.back.retain(|path| keep(path));
        self.forward.retain(|path| keep(path));
    }

    pub(super) fn clear(&mut self) {
        self.back.clear();
        self.forward.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_back_forward_and_new_branches() {
        let mut navigation = LibraryNavigation::default();

        navigation.visit(Some("first"), "second");
        navigation.visit(Some("second"), "third");
        assert_eq!(navigation.go_back(Some("third")).as_deref(), Some("second"));
        assert_eq!(navigation.go_back(Some("second")).as_deref(), Some("first"));
        assert_eq!(
            navigation.go_forward(Some("first")).as_deref(),
            Some("second")
        );

        navigation.visit(Some("second"), "branch");
        assert!(!navigation.can_go_forward());
        assert_eq!(
            navigation.go_back(Some("branch")).as_deref(),
            Some("second")
        );
    }

    #[test]
    fn ignores_reselecting_the_current_path() {
        let mut navigation = LibraryNavigation::default();

        navigation.visit(Some("first"), "first");

        assert!(!navigation.can_go_back());
        assert!(!navigation.can_go_forward());
    }

    #[test]
    fn selecting_after_the_current_path_disappears_clears_the_forward_branch() {
        let mut navigation = LibraryNavigation::default();
        navigation.visit(Some("first"), "second");
        assert_eq!(navigation.go_back(Some("second")).as_deref(), Some("first"));

        navigation.visit(None, "branch");

        assert!(!navigation.can_go_forward());
    }

    #[test]
    fn removes_paths_that_are_no_longer_available() {
        let mut navigation = LibraryNavigation::default();
        navigation.visit(Some("first"), "second");
        navigation.visit(Some("second"), "third");
        assert_eq!(navigation.go_back(Some("third")).as_deref(), Some("second"));

        navigation.retain(|path| path == "first");

        assert_eq!(navigation.go_back(None).as_deref(), Some("first"));
        assert!(!navigation.can_go_back());
        assert!(!navigation.can_go_forward());
    }
}
