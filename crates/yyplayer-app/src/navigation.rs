/// Session-local page history. Returning consumes an entry instead of recording
/// another visit, so repeated Back actions always walk toward the initial page.
#[derive(Default)]
pub(super) struct History {
    current: i32,
    previous: Vec<i32>,
}

impl History {
    pub(super) fn current(&self) -> i32 {
        self.current
    }

    pub(super) fn can_go_back(&self) -> bool {
        !self.previous.is_empty()
    }

    pub(super) fn visit(&mut self, page: i32) -> bool {
        if !(0..=6).contains(&page) || page == self.current {
            return false;
        }
        self.previous.push(self.current);
        self.current = page;
        true
    }

    pub(super) fn back(&mut self) -> bool {
        if let Some(page) = self.previous.pop() {
            self.current = page;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn visits_return_in_reverse_order_without_recording_back() {
        let mut history = History::default();
        for page in [6, 3, 2, 5, 1, 4] {
            assert!(history.visit(page));
        }
        for page in [1, 5, 2, 3, 6, 0] {
            assert!(history.back());
            assert_eq!(history.current(), page);
        }
        assert!(!history.can_go_back());
        assert!(!history.back());
        assert_eq!(history.current(), 0);
    }

    #[test]
    fn same_page_and_invalid_pages_do_not_pollute_history() {
        let mut history = History::default();
        assert!(!history.visit(0));
        assert!(!history.visit(-1));
        assert!(!history.visit(7));
        assert!(!history.can_go_back());
        assert!(history.visit(6));
        assert!(!history.visit(6));
        assert!(history.back());
        assert_eq!(history.current(), 0);
        assert!(!history.can_go_back());
    }

    #[test]
    fn revisiting_a_page_and_visiting_after_back_keep_the_actual_route() {
        let mut history = History::default();
        for page in [6, 3, 6] {
            history.visit(page);
        }
        assert!(history.back());
        assert_eq!(history.current(), 3);
        assert!(history.visit(5));
        for page in [3, 6, 0] {
            assert!(history.back());
            assert_eq!(history.current(), page);
        }
        assert!(!history.back());
    }
}
