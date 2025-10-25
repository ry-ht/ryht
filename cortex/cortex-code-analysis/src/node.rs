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
