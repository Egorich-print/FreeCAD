//! Document history — Fusion-like timeline / undo-redo.
//! Ponytail: simplest stack that works, no dependencies.

use crate::document::{Document, SceneObject};

/// A single reversible transaction (Fusion Timeline step)
#[derive(Debug, Clone)]
pub struct Transaction {
    pub label: String,
    pub before: Vec<SceneObject>,
    pub after: Vec<SceneObject>,
}

impl Transaction {
    pub fn new(
        label: impl Into<String>,
        before: Vec<SceneObject>,
        after: Vec<SceneObject>,
    ) -> Self {
        Self {
            label: label.into(),
            before,
            after,
        }
    }
}

/// Linear undo/redo history
#[derive(Debug, Default)]
pub struct History {
    /// Current document snapshot
    current: Document,
    /// Undo stack (oldest first, last = most recent)
    undo: Vec<Transaction>,
    /// Redo stack
    redo: Vec<Transaction>,
}

impl History {
    pub fn new(doc: Document) -> Self {
        Self {
            current: doc,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn document(&self) -> &Document {
        &self.current
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Apply a transaction: save before/after, push to undo, clear redo
    pub fn commit(&mut self, label: impl Into<String>, new_doc: Document) {
        let tx = Transaction::new(label, self.current.objects.clone(), new_doc.objects.clone());
        self.undo.push(tx);
        self.redo.clear();
        self.current = new_doc;
    }

    pub fn undo(&mut self) -> Option<&str> {
        let tx = self.undo.pop()?;
        self.current.objects = tx.before.clone();
        self.redo.push(tx);
        Some(&self.redo.last().unwrap().label)
    }

    pub fn redo(&mut self) -> Option<&str> {
        let tx = self.redo.pop()?;
        self.current.objects = tx.after.clone();
        self.undo.push(tx);
        Some(&self.undo.last().unwrap().label)
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Fusion timeline: ordered labels oldest → newest
    pub fn timeline(&self) -> Vec<String> {
        self.undo.iter().map(|t| t.label.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{ObjectId, Placement};

    fn doc_with(n: u32) -> Document {
        let mut d = Document::new();
        d.add(SceneObject {
            id: ObjectId(n),
            label: format!("Obj{n}"),
            type_name: "Part::Feature".into(),
            shape_index: Some(0),
            placement: Placement::identity(),
            visible: true,
        });
        d
    }

    #[test]
    fn commit_and_undo_redo() {
        let mut h = History::new(Document::new());
        h.commit("Create Box", doc_with(1));
        assert_eq!(h.undo_len(), 1);
        assert_eq!(h.document().objects.len(), 1);
        h.commit("Create Chamfer", doc_with(2));
        assert_eq!(h.undo_len(), 2);
        // undo last -> back to 1 obj
        let lbl = h.undo().unwrap();
        assert_eq!(lbl, "Create Chamfer");
        assert_eq!(h.document().objects.len(), 1);
        assert_eq!(h.document().objects[0].id.0, 1);
        assert!(h.can_redo());
        // redo
        let lbl2 = h.redo().unwrap();
        assert_eq!(lbl2, "Create Chamfer");
        assert_eq!(h.document().objects[0].id.0, 2);
        // undo again and commit new branch clears redo
        h.undo();
        assert_eq!(h.redo_len(), 1);
        h.commit("New Sketch", doc_with(3));
        assert_eq!(h.redo_len(), 0);
        assert_eq!(h.timeline(), vec!["Create Box", "New Sketch"]);
    }

    #[test]
    fn empty_undo_redo() {
        let mut h = History::new(Document::new());
        assert!(h.undo().is_none());
        assert!(h.redo().is_none());
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }
}
