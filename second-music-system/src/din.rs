//! Descriptively Indented Nodes

use std::mem::take;

#[cfg(test)]
macro_rules! node {
    ($lineno:expr, $items:expr) => {
        node!($lineno, $items, [])
    };
    ($lineno:expr, $items:expr, $children:expr) => {
        DinNode {
            lineno: $lineno,
            items: $items.into_iter().map(|x| x.to_string()).collect(),
            children: $children.into_iter().map(Some).collect(),
        }
    };
}

#[cfg(test)]
mod test;

pub(crate) mod macros;

#[derive(Debug, PartialEq)]
struct DinParser<'a> {
    rem: &'a [u8],
    lineno: usize,
    indentation_levels: Vec<usize>,
    nodes_to_yield: Vec<ParseItem>,
}

#[derive(Debug, PartialEq, Eq)]
enum ParseItem {
    BeginNode(Vec<String>, usize),
    EndNode,
}

impl DinParser<'_> {
    pub fn new(bytes: &[u8]) -> DinParser {
        DinParser {
            rem: bytes,
            lineno: 1,
            indentation_levels: vec![],
            nodes_to_yield: vec![],
        }
    }
}

impl Iterator for DinParser<'_> {
    type Item = Result<ParseItem, String>;
    fn next(&mut self) -> Option<Self::Item> {
        while !self.rem.is_empty() || !self.nodes_to_yield.is_empty() {
            if let Some(node) = self.nodes_to_yield.pop() {
                return Some(Ok(node));
            }
            let mut indentation_level = 0;
            while let Some(x) = self.rem.first() {
                if *x == b' ' || *x == b'\t' {
                    self.rem = &self.rem[1..];
                    indentation_level += 1;
                } else {
                    break;
                }
            }
            let end_index = self
                .rem
                .iter()
                .enumerate()
                .find(|(_i, c)| **c == b'\n' || **c == b'\r')
                .map(|(i, _)| i)
                .unwrap_or(self.rem.len());
            let line = &self.rem[..end_index];
            let parsed = match shellish_parse::parse(
                unsafe { std::str::from_utf8_unchecked(line) },
                true,
            ) {
                Ok(x) => x,
                Err(x) => {
                    return Some(Err(format!("line {}: {}", self.lineno, x)))
                }
            };
            if !parsed.is_empty() {
                self.nodes_to_yield
                    .push(ParseItem::BeginNode(parsed, self.lineno));
                while let Some(prev_level) = self.indentation_levels.last() {
                    if *prev_level >= indentation_level {
                        self.nodes_to_yield.push(ParseItem::EndNode);
                        self.indentation_levels.pop();
                    } else {
                        break;
                    }
                }
                self.indentation_levels.push(indentation_level);
            }
            self.rem = &self.rem[end_index..];
            while let Some(x) = self.rem.first() {
                if *x == b'\r' || *x == b'\n' {
                    if *x == b'\n' {
                        self.lineno += 1;
                    }
                    self.rem = &self.rem[1..];
                } else {
                    break;
                }
            }
        }
        if self.indentation_levels.pop().is_some() {
            return Some(Ok(ParseItem::EndNode));
        }
        None
    }
}

#[derive(Debug, PartialEq)]
pub struct DinNode {
    pub items: Vec<String>,
    pub children: Vec<Option<DinNode>>,
    pub lineno: usize,
}

pub fn parse_din(src: &str) -> Result<Vec<DinNode>, String> {
    let mut ret: Vec<DinNode> = vec![];
    let mut stack: Vec<DinNode> = vec![];
    for item in DinParser::new(src.as_bytes()) {
        let item = item?;
        match item {
            ParseItem::BeginNode(items, lineno) => stack.push(DinNode {
                items,
                children: vec![],
                lineno,
            }),
            ParseItem::EndNode => {
                let endut = stack.pop().unwrap();
                match stack.last_mut() {
                    Some(x) => x.children.push(Some(endut)),
                    None => ret.push(endut),
                }
            }
        }
    }
    assert_eq!(stack.len(), 0);
    Ok(ret)
}

impl DinNode {
    /// Consume and yield all children.
    pub fn consume_children(
        &'_ mut self,
    ) -> impl '_ + Iterator<Item = DinNode> {
        self.consume_predicated_children(|_| true)
    }
    /// Consume and yield all children that match the predicate.
    pub fn consume_predicated_children<'a>(
        &'a mut self,
        predicate: impl 'a + Fn(&DinNode) -> bool,
    ) -> impl 'a + Iterator<Item = DinNode> {
        self.children.iter_mut().filter_map(move |node| {
            if node.as_ref().map(&predicate).unwrap_or(false) {
                take(node)
            } else {
                None
            }
        })
    }
    /// Consume and yield all children named `name`.
    pub fn consume_prefixed_children<'a>(
        &'a mut self,
        name: &'a str,
    ) -> impl 'a + Iterator<Item = DinNode> {
        self.consume_predicated_children(move |x| x.items[0].as_str() == name)
    }
    /// Consume and yield all children whose names appear in the list.
    pub fn consume_designated_children<'a>(
        &'a mut self,
        names: &'a [&'a str],
    ) -> impl 'a + Iterator<Item = DinNode> {
        self.consume_predicated_children(|x| {
            names.iter().any(|y| *y == x.items[0].as_str())
        })
    }
    /// If there is one child named `name`, consume and yield it. If there are
    /// none, return `Ok(None)`. If there was more than one, return an error.
    pub fn consume_optional_prefixed_child(
        &mut self,
        name: &str,
    ) -> Result<Option<DinNode>, String> {
        let lineno = self.lineno;
        let mut iter = self.consume_prefixed_children(name);
        let ret = iter.next();
        // duplicate???
        if iter.next().is_some() {
            Err(format!(
                "line {}: multiple {name:?} children seen, only one is \
                allowed",
                lineno
            ))
        } else {
            Ok(ret)
        }
    }
    /// If there is one child named `name`, consume and yield it. If there are
    /// none, an error. If there was more than one, return an error.
    pub fn consume_required_prefixed_child(
        &mut self,
        name: &str,
    ) -> Result<DinNode, String> {
        let lineno = self.lineno;
        let mut iter = self.consume_prefixed_children(name);
        let ret = iter.next();
        // duplicate???
        if iter.next().is_some() {
            Err(format!(
                "line {}: multiple {name:?} children seen, exactly one is \
                allowed",
                lineno
            ))
        } else if let Some(ret) = ret {
            Ok(ret)
        } else {
            Err(format!("line {}: one {name:?} child is required", lineno))
        }
    }
    /// If there are any unconsumed children left, return an error saying that
    /// those children were not understood. If there are none, return `Ok(())`.
    pub fn finish_parsing_children(self) -> Result<(), String> {
        let mut count = 0;
        let mut error =
            "the following nodes were not understood:\n".to_string();
        for child in self.children.into_iter().flatten() {
            count += 1;
            error.push_str(&format!(
                "\tline {}: {:?}\n",
                child.lineno, child.items[0]
            ));
        }
        if count > 0 {
            return Err(error);
        }
        Ok(())
    }
    pub fn any_children_left(&self) -> bool {
        self.children
            .iter()
            .filter_map(|x| x.as_ref())
            .next()
            .is_some()
    }
}
