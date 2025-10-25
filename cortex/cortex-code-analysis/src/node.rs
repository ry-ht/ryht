//! AST node abstraction layer.
//!
//! This module provides a thin wrapper around tree-sitter's Node type,
//! offering a more ergonomic interface for tree traversal and analysis.

use tree_sitter::Node as TSNode;
use tree_sitter::TreeCursor;

use crate::traits::Search;

/// An AST node wrapper providing convenient tree traversal methods.
#[derive(Clone, Copy, Debug)]
pub struct Node<'a>(pub(crate) TSNode<'a>);

impl<'a> Node<'a> {
    /// Create a new Node from a tree-sitter node
    pub fn new(node: TSNode<'a>) -> Self {
        Self(node)
    }

    /// Get the underlying tree-sitter node
    pub fn inner(&self) -> TSNode<'a> {
        self.0
    }

    /// Checks if a node represents a syntax error or contains any syntax errors
    /// anywhere within it.
    pub fn has_error(&self) -> bool {
        self.0.has_error()
    }

    /// Get the unique ID of this node
    pub(crate) fn id(&self) -> usize {
        self.0.id()
    }

    /// Get the node kind as a string
    pub fn kind(&self) -> &'static str {
        self.0.kind()
    }

    /// Get the node kind as a numeric ID
    pub fn kind_id(&self) -> u16 {
        self.0.kind_id()
    }

    /// Get the UTF-8 text content of this node
    pub fn utf8_text(&self, data: &'a [u8]) -> Option<&'a str> {
        self.0.utf8_text(data).ok()
    }

    /// Get the start byte offset
    pub fn start_byte(&self) -> usize {
        self.0.start_byte()
    }

    /// Get the end byte offset
    pub fn end_byte(&self) -> usize {
        self.0.end_byte()
    }

    /// Get the start position as (row, column)
    pub fn start_position(&self) -> (usize, usize) {
        let pos = self.0.start_position();
        (pos.row, pos.column)
    }

    /// Get the end position as (row, column)
    pub fn end_position(&self) -> (usize, usize) {
        let pos = self.0.end_position();
        (pos.row, pos.column)
    }

    /// Get the start row (0-indexed)
    pub fn start_row(&self) -> usize {
        self.0.start_position().row
    }

    /// Get the end row (0-indexed)
    pub fn end_row(&self) -> usize {
        self.0.end_position().row
    }

    /// Get the parent node
    pub fn parent(&self) -> Option<Node<'a>> {
        self.0.parent().map(Node)
    }

    /// Check if this node has a sibling with the given kind ID
    #[inline(always)]
    pub fn has_sibling(&self, id: u16) -> bool {
        self.0.parent().is_some_and(|parent| {
            parent
                .children(&mut parent.walk())
                .any(|child| child.kind_id() == id)
        })
    }

    /// Get the previous sibling
    pub fn previous_sibling(&self) -> Option<Node<'a>> {
        self.0.prev_sibling().map(Node)
    }

    /// Get the next sibling
    pub fn next_sibling(&self) -> Option<Node<'a>> {
        self.0.next_sibling().map(Node)
    }

    /// Check if this node has a child with the given kind ID
    #[inline(always)]
    pub fn is_child(&self, id: u16) -> bool {
        self.0
            .children(&mut self.0.walk())
            .any(|child| child.kind_id() == id)
    }

    /// Get the number of children
    pub fn child_count(&self) -> usize {
        self.0.child_count()
    }

    /// Get a child by field name
    pub fn child_by_field_name(&self, name: &str) -> Option<Node<'a>> {
        self.0.child_by_field_name(name).map(Node)
    }

    /// Get a child by index
    pub fn child(&self, pos: usize) -> Option<Node<'a>> {
        self.0.child(pos).map(Node)
    }

    /// Get an iterator over all children
    pub fn children(&self) -> impl ExactSizeIterator<Item = Node<'a>> + use<'a> {
        let mut cursor = self.cursor();
        cursor.goto_first_child();
        (0..self.child_count()).map(move |_| {
            let result = cursor.node();
            cursor.goto_next_sibling();
            result
        })
    }

    /// Get a cursor for tree traversal
    pub fn cursor(&self) -> Cursor<'a> {
        Cursor(self.0.walk())
    }

    /// Count ancestors matching a predicate until a stop condition
    pub fn count_specific_ancestors(
        &self,
        check: fn(&Node) -> bool,
        stop: fn(&Node) -> bool,
    ) -> usize {
        let mut count = 0;
        let mut node = *self;
        while let Some(parent) = node.parent() {
            if stop(&parent) {
                break;
            }
            if check(&parent) {
                count += 1;
            }
            node = parent;
        }
        count
    }

    /// Check if node has specific ancestor chain
    pub fn has_ancestors(&self, typ: fn(&Node) -> bool, typs: fn(&Node) -> bool) -> bool {
        let mut res = false;
        let mut node = *self;
        if let Some(parent) = node.parent() {
            if typ(&parent) {
                node = parent;
            }
        }
        if let Some(parent) = node.parent() {
            if typs(&parent) {
                res = true;
            }
        }
        res
    }

    /// Get all ancestors of this node
    pub fn ancestors(&self) -> impl Iterator<Item = Node<'a>> {
        let mut current = *self;
        std::iter::from_fn(move || {
            current.parent().map(|parent| {
                current = parent;
                parent
            })
        })
    }

    /// Get the depth of this node in the tree (root = 0)
    pub fn depth(&self) -> usize {
        self.ancestors().count()
    }

    /// Get the node path from root to this node
    pub fn path(&self) -> Vec<Node<'a>> {
        let mut path: Vec<Node<'a>> = self.ancestors().collect();
        path.reverse();
        path.push(*self);
        path
    }

    /// Get the node path as kind strings
    pub fn path_kinds(&self) -> Vec<&'static str> {
        self.path().iter().map(|n| n.kind()).collect()
    }

    /// Find the first ancestor matching a predicate
    pub fn find_ancestor(&self, pred: fn(&Node) -> bool) -> Option<Node<'a>> {
        self.ancestors().find(|n| pred(n))
    }

    /// Find the first ancestor of a specific kind
    pub fn find_ancestor_of_kind(&self, kind: &str) -> Option<Node<'a>> {
        self.ancestors().find(|n| n.kind() == kind)
    }

    /// Get all siblings (including self)
    pub fn siblings(&self) -> Vec<Node<'a>> {
        if let Some(parent) = self.parent() {
            parent.children().collect()
        } else {
            vec![*self]
        }
    }

    /// Get all siblings excluding self
    pub fn siblings_excluding_self(&self) -> Vec<Node<'a>> {
        let self_id = self.id();
        self.siblings()
            .into_iter()
            .filter(|n| n.id() != self_id)
            .collect()
    }

    /// Get the index of this node among its siblings
    pub fn sibling_index(&self) -> usize {
        if let Some(parent) = self.parent() {
            parent
                .children()
                .position(|n| n.id() == self.id())
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Get all descendants of this node (breadth-first)
    pub fn descendants_bfs(&self) -> Vec<Node<'a>> {
        let mut result = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(*self);

        while let Some(node) = queue.pop_front() {
            for child in node.children() {
                result.push(child);
                queue.push_back(child);
            }
        }

        result
    }

    /// Get all descendants of this node (depth-first)
    pub fn descendants_dfs(&self) -> Vec<Node<'a>> {
        let mut result = Vec::new();
        let mut stack = vec![*self];

        while let Some(node) = stack.pop() {
            let children: Vec<_> = node.children().collect();
            for child in children.into_iter().rev() {
                result.push(child);
                stack.push(child);
            }
        }

        result
    }

    /// Find all descendants matching a predicate
    pub fn find_descendants(&self, pred: fn(&Node) -> bool) -> Vec<Node<'a>> {
        let mut result = Vec::new();
        self.act_on_node(&mut |node| {
            if pred(node) {
                result.push(*node);
            }
        });
        result
    }

    /// Find all descendants of a specific kind
    pub fn find_descendants_of_kind(&self, kind: &'static str) -> Vec<Node<'a>> {
        let mut result = Vec::new();
        self.act_on_node(&mut |node| {
            if node.kind() == kind {
                result.push(*node);
            }
        });
        result
    }

    /// Check if this node is an ancestor of another node
    pub fn is_ancestor_of(&self, other: &Node<'a>) -> bool {
        other.ancestors().any(|n| n.id() == self.id())
    }

    /// Check if this node is a descendant of another node
    pub fn is_descendant_of(&self, other: &Node<'a>) -> bool {
        other.is_ancestor_of(self)
    }

    /// Get the lowest common ancestor with another node
    pub fn lowest_common_ancestor(&self, other: &Node<'a>) -> Option<Node<'a>> {
        let self_ancestors: Vec<_> = self.ancestors().collect();
        let other_ancestors: Vec<_> = other.ancestors().collect();

        for self_anc in self_ancestors.iter() {
            for other_anc in other_ancestors.iter() {
                if self_anc.id() == other_anc.id() {
                    return Some(*self_anc);
                }
            }
        }

        None
    }

    /// Get the byte range of this node
    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.start_byte()..self.end_byte()
    }

    /// Get the byte length of this node
    pub fn byte_len(&self) -> usize {
        self.end_byte() - self.start_byte()
    }

    /// Get the line count of this node (inclusive)
    pub fn line_count(&self) -> usize {
        self.end_row().saturating_sub(self.start_row()) + 1
    }

    /// Check if this node is a leaf (has no children)
    pub fn is_leaf(&self) -> bool {
        self.child_count() == 0
    }

    /// Check if this node spans multiple lines
    pub fn is_multiline(&self) -> bool {
        self.start_row() != self.end_row()
    }

    /// Check if this node is named (has a field name in the grammar)
    pub fn is_named(&self) -> bool {
        self.0.is_named()
    }

    /// Get all named children
    pub fn named_children(&self) -> Vec<Node<'a>> {
        self.children().filter(|n| n.is_named()).collect()
    }

    /// Get the number of named children
    pub fn named_child_count(&self) -> usize {
        self.0.named_child_count()
    }

    /// Get a named child by index
    pub fn named_child(&self, pos: usize) -> Option<Node<'a>> {
        self.0.named_child(pos).map(Node)
    }
}

/// A tree cursor for manual tree traversal.
#[derive(Clone)]
pub struct Cursor<'a>(pub(crate) TreeCursor<'a>);

impl<'a> Cursor<'a> {
    /// Reset cursor to a specific node
    pub fn reset(&mut self, node: &Node<'a>) {
        self.0.reset(node.0);
    }

    /// Move cursor to next sibling
    pub fn goto_next_sibling(&mut self) -> bool {
        self.0.goto_next_sibling()
    }

    /// Move cursor to first child
    pub fn goto_first_child(&mut self) -> bool {
        self.0.goto_first_child()
    }

    /// Get the current node
    pub fn node(&self) -> Node<'a> {
        Node(self.0.node())
    }
}

impl<'a> Search<'a> for Node<'a> {
    fn first_occurrence(&self, pred: fn(u16) -> bool) -> Option<Node<'a>> {
        let mut cursor = self.cursor();
        let mut stack = Vec::new();
        let mut children = Vec::new();

        stack.push(*self);

        while let Some(node) = stack.pop() {
            if pred(node.kind_id()) {
                return Some(node);
            }
            cursor.reset(&node);
            if cursor.goto_first_child() {
                loop {
                    children.push(cursor.node());
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                for child in children.drain(..).rev() {
                    stack.push(child);
                }
            }
        }

        None
    }

    fn act_on_node(&self, action: &mut dyn FnMut(&Node<'a>)) {
        let mut cursor = self.cursor();
        let mut stack = Vec::new();
        let mut children = Vec::new();

        stack.push(*self);

        while let Some(node) = stack.pop() {
            action(&node);
            cursor.reset(&node);
            if cursor.goto_first_child() {
                loop {
                    children.push(cursor.node());
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                for child in children.drain(..).rev() {
                    stack.push(child);
                }
            }
        }
    }

    fn first_child(&self, pred: fn(u16) -> bool) -> Option<Node<'a>> {
        self.children().find(|&child| pred(child.kind_id()))
    }

    fn act_on_child(&self, action: &mut dyn FnMut(&Node<'a>)) {
        for child in self.children() {
            action(&child);
        }
    }
}
