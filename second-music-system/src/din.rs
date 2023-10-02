//! Descriptively Indented Nodes

#[cfg(test)]
macro_rules! node {
    ($lineno:expr, $items:expr) => {
        node!($lineno, $items, [])
    };
    ($lineno:expr, $items:expr, $children:expr) => {
        DinNode {
            lineno: $lineno,
            items: $items.into_iter().map(|x| x.to_string()).collect(),
            children: $children.into_iter().collect(),
        }
    };
}

#[cfg(test)]
mod test;

#[derive(Debug,PartialEq)]
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
        while self.rem.len() > 0 || self.nodes_to_yield.len() > 0 {
            if let Some(node) = self.nodes_to_yield.pop() {
                return Some(Ok(node))
            }
            let mut indentation_level = 0;
            while let Some(x) = self.rem.first() {
                if *x == b' ' || *x == b'\t' {
                    self.rem = &self.rem[1..];
                    indentation_level += 1;
                }
                else { break }
            }
            let end_index = self.rem.iter().enumerate().find(|(_i,c)| **c == b'\n' || **c == b'\r').map(|(i,_)| i).unwrap_or(self.rem.len());
            let line = &self.rem[..end_index];
            let parsed = match shellish_parse::parse(unsafe { std::str::from_utf8_unchecked(line) }, true) {
                Ok(x) => x,
                Err(x) => return Some(Err(format!("line {}: {}", self.lineno, x))),
            };
            if parsed.len() > 0 {
                self.nodes_to_yield.push(ParseItem::BeginNode(parsed, self.lineno));
                while let Some(prev_level) = self.indentation_levels.last() {
                    if *prev_level >= indentation_level {
                        self.nodes_to_yield.push(ParseItem::EndNode);
                        self.indentation_levels.pop();
                    }
                    else { break }
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
                }
                else { break }
            }
        }
        if let Some(_) = self.indentation_levels.pop() {
            return Some(Ok(ParseItem::EndNode))
        }
        None
    }
}

#[derive(Debug,PartialEq)]
pub struct DinNode {
    pub items: Vec<String>,
    pub children: Vec<DinNode>,
    pub lineno: usize,
}

pub fn parse_din(src: &str) -> Result<Vec<DinNode>, String> {
    let mut ret: Vec<DinNode> = vec![];
    let mut stack: Vec<DinNode> = vec![];
    for item in DinParser::new(src.as_bytes()) {
        let item = item?;
        match item {
            ParseItem::BeginNode(items, lineno) =>
                stack.push(DinNode { items, children: vec![], lineno }),
            ParseItem::EndNode => {
                let endut = stack.pop().unwrap();
                match stack.last_mut() {
                    Some(x) => x.children.push(endut),
                    None => ret.push(endut),
                }
            },
        }
    }
    assert_eq!(stack.len(), 0);
    Ok(ret)
}
