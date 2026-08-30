use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct MarkdownCodeBlocks(Rc<RefCell<CodeBlockState>>);

#[derive(Default)]
struct CodeBlockState {
    document: Arc<str>,
    expanded: HashSet<usize>,
    overflowing: HashSet<usize>,
}

impl MarkdownCodeBlocks {
    pub(super) fn set_document(&self, document: Arc<str>) {
        let mut state = self.0.borrow_mut();
        if state.document.as_ref() != document.as_ref() {
            state.expanded.clear();
            state.overflowing.clear();
        }
        state.document = document;
    }

    pub fn reset(&self) {
        let mut state = self.0.borrow_mut();
        state.expanded.clear();
        state.overflowing.clear();
    }

    pub(super) fn is_expanded(&self, code_id: usize) -> bool {
        self.0.borrow().expanded.contains(&code_id)
    }

    pub(super) fn is_overflowing(&self, code_id: usize) -> bool {
        self.0.borrow().overflowing.contains(&code_id)
    }

    pub(super) fn set_expanded(&self, code_id: usize, expanded: bool) -> bool {
        let mut state = self.0.borrow_mut();
        if expanded {
            state.expanded.insert(code_id)
        } else {
            state.expanded.remove(&code_id)
        }
    }

    pub(super) fn set_overflowing(&self, code_id: usize, overflowing: bool) -> bool {
        let mut state = self.0.borrow_mut();
        if overflowing {
            state.overflowing.insert(code_id)
        } else {
            state.overflowing.remove(&code_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_documents_resets_code_block_state() {
        let code_blocks = MarkdownCodeBlocks::default();
        code_blocks.set_document(Arc::from("first"));
        code_blocks.set_overflowing(0, true);
        code_blocks.set_expanded(0, true);

        code_blocks.set_document(Arc::from("second"));

        assert!(!code_blocks.is_overflowing(0));
        assert!(!code_blocks.is_expanded(0));
    }

    #[test]
    fn same_document_preserves_code_block_state() {
        let code_blocks = MarkdownCodeBlocks::default();
        code_blocks.set_document(Arc::from("same"));
        code_blocks.set_overflowing(1, true);
        code_blocks.set_expanded(1, true);

        code_blocks.set_document(Arc::from("same"));

        assert!(code_blocks.is_overflowing(1));
        assert!(code_blocks.is_expanded(1));
    }

    #[test]
    fn expanded_code_blocks_can_be_collapsed() {
        let code_blocks = MarkdownCodeBlocks::default();
        code_blocks.set_expanded(0, true);

        assert!(code_blocks.set_expanded(0, false));
        assert!(!code_blocks.is_expanded(0));
    }
}
