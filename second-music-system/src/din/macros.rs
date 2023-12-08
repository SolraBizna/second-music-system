#[macro_export]
macro_rules! match_din_pattern {
    ($attempt:lifetime, $items:path, ) => {{}};
    // !"foo" => match if next item is not "foo" (and don't consume it)
    ($attempt:lifetime, $items:path, !$homer:literal $($rest:tt)*) => {
        match $items.peek() {
            None => break $attempt false,
            Some(x) => {
                if x == &$homer {
                    break $attempt false;
                } else {
                    $crate::match_din_pattern!($attempt, $items, $($rest)*)
                }
            }
        }
    };
    // "foo" => match if next item is "foo"
    ($attempt:lifetime, $items:path, $homer:literal $($rest:tt)*) => {
        match $items.next() {
            None => break $attempt false,
            Some(x) => {
                if x != $homer {
                    break $attempt false;
                } else {
                    $crate::match_din_pattern!($attempt, $items, $($rest)*)
                }
            }
        }
    };
    // foo=("bar"|"baz"|...) => match if next item is one of "bar", "baz", ...
    // and assign it to the variable foo
    ($attempt:lifetime, $items:path, $name:path=($($homer:literal)|+) $($rest:tt)*) => {
        match $items.next() {
            None => break $attempt false,
            Some(x) => {
                if ![$($homer),+].iter().any(|homer| x == homer)  {
                    break $attempt false;
                } else {
                    $name = Some(compact_str::ToCompactString::to_compact_string(x));
                    $crate::match_din_pattern!($attempt, $items, $($rest)*)
                }
            }
        }
    };
    // foo=* => match if there is a next item, and assign it to the variable foo
    ($attempt:lifetime, $items:path, $homer:path=* $($rest:tt)*) => {
        match $items.next() {
            None => break $attempt false,
            Some(x) => {
                $homer = Some(compact_str::ToCompactString::to_compact_string(x));
                $crate::match_din_pattern!($attempt, $items, $($rest)*)
            }
        }
    };
    // "[...]" => if ... matches, consume it. if it doesn't, consume nothing.
    // always matches.
    // "foo=[...]"
    // as above, but puts whether it matched or not into foo
    ($attempt:lifetime, $items:path, $($did_match:path =)? [$($subpattern:tt)+] $($rest:tt)*) => {{
        let mut parallel_items = $items.clone();
        let did_match = 'subattempt: {
            $crate::match_din_pattern!('subattempt, parallel_items, $($subpattern)+);
            true
        };
        $($did_match = did_match;)?
        if did_match {
            $items = parallel_items;
        } else {
            $crate::unmatch_din_subpattern!($($subpattern)+);
        }
        $crate::match_din_pattern!($attempt, $items, $($rest)*)
    }};
}

// called if a subpattern didn't match
#[macro_export]
macro_rules! unmatch_din_subpattern {
    () => {{}};
    (!$homer:literal $($rest:tt)*) => {
        $crate::unmatch_din_subpattern!($($rest)*);
    };
    ($homer:literal $($rest:tt)*) => {
        $crate::unmatch_din_subpattern!($($rest)*);
    };
    ($name:path=($($_homer:literal)|+) $($rest:tt)*) => {{
        $name = None;
        $crate::unmatch_din_subpattern!($($rest)*);
    }};
    ($homer:path=* $($rest:tt)*) => {{
        $homer = None;
        $crate::unmatch_din_subpattern!($($rest)*);
    }};
}

#[macro_export]
macro_rules! describe_din_node_pattern {
    ($($_pattern:tt)*) => {
        "expected stuff, found TODO".to_string()
    };
}

#[macro_export]
macro_rules! parse_din_node {
    ($node:path, $($pattern:tt)+) => {{
        let success = 'attempt: {
            let mut items = $node.items.iter().peekable();
            $crate::match_din_pattern!('attempt, items, $($pattern)+);
            if items.next().is_some() {
                // there was extra stuff...
                false
            } else {
                true
            }
        };
        if success { Ok(()) }
        else { Err($crate::describe_din_node_pattern!($($pattern)+)) }
    }}
}

#[macro_export]
macro_rules! parse_optional_prefixed_child {
    ($node:path, $first:literal $($pattern:tt)*) => {{
        match $node.consume_optional_prefixed_child($first) {
            Err(x) => Err(x),
            Ok(None) => Ok(false),
            Ok(Some(child)) => {
                if !child.children.is_empty() {
                    Err(format!(concat!("line {}: \"", $first, "\" must not have children (check indentation)"), $node.lineno))
                } else {
                    parse_din_node!(child, $first $($pattern)*).map(|_| true)
                }
            }
        }
    }}
}

#[macro_export]
macro_rules! parse_mandatory_prefixed_child {
    ($node:path, $first:literal $($pattern:tt)*) => {{
        match $node.consume_required_prefixed_child($first) {
            Err(x) => Err(x)
            Ok(child) => {
                if !child.children.is_empty() {
                    Err(format!(concat!("line {}: \"", $first, "\" must not have children (check indentation)"), $node.lineno))
                } else {
                    parse_din_node!(child, $first $($pattern)*)
                }
            }
        }
    }}
}

#[cfg(test)]
#[path = "macro_test.rs"]
mod test;
