use super::*;

#[derive(Debug)]
enum ExprNode<'a> {
    Subexpression(Vec<PredicateOp>),
    UnOp { op: &'a str },
    BinOp { op: &'a str, precedence: i32 },
    StringOrNumber(StringOrNumber),
}

impl ExprNode<'_> {
    fn push_subexpression(self, ops: &mut Vec<PredicateOp>) {
        match self {
            ExprNode::Subexpression(mut subexpr) => ops.append(&mut subexpr),
            ExprNode::StringOrNumber(son) => ops.push(PredicateOp::PushConst(son)),
            _ => panic!("internal error: push_subexpression only valid for ExprNode::Subexpression and ExprNode::StringOrNumber"),
        }
    }
}

// BINARY OPERATOR PRECEDENCE
// 1: exponent, logarithm, atan2
// 2: multiplication, division, modulo
// 3: addition, subtraction
// 4: min, max
// 5: comparisons
// 6: and
// 7: or

fn parse_partial(
    it: &mut std::vec::IntoIter<&str>,
    top_level: bool,
) -> Result<Vec<PredicateOp>, String> {
    let mut partial = Vec::new();
    loop {
        let x = it.next();
        let x = match x {
            None => {
                if top_level {
                    break;
                } else {
                    return Err("unbalanced parentheses in expression (not enough \")\")".to_string());
                }
            }
            Some(")") => {
                if !top_level {
                    break;
                } else {
                    return Err("unbalanced parentheses in expression (too many \")\")".to_string());
                }
            }
            Some(x) => x,
        };
        match x {
            "(" => {
                let subops = parse_partial(it, false)?;
                partial.push(ExprNode::Subexpression(subops));
            },
            "=" | "≠" | ">" | "≥" | "<" | "≤" => {
                partial.push(ExprNode::BinOp { op: x, precedence: 5 });
            },
            "min" | "max" => {
                partial.push(ExprNode::BinOp { op: x, precedence: 4 });
            },
            "-" => {
                match partial.last() {
                    None | Some(ExprNode::BinOp { .. }) => partial.push(ExprNode::UnOp { op: "-" }),
                    _ => partial.push(ExprNode::BinOp { op: "-", precedence: 3 }),
                }
            },
            "+" => {
                partial.push(ExprNode::BinOp { op: x, precedence: 3 });
            },
            "*" | "/" | "//" | "%" => {
                partial.push(ExprNode::BinOp { op: x, precedence: 2 });
            },
            "^" | "log" | "atan2" => {
                partial.push(ExprNode::BinOp { op: x, precedence: 1 });
            },
            "$" | "not" | "sin" | "cos" | "tan" | "asin" | "acos"
            | "atan" | "ln" | "exp" | "floor" | "ceil" | "abs" | "sign" => {
                partial.push(ExprNode::UnOp { op: x });
            },
            "&" | "|" | "!" | "~" => {
                return Err(format!("there is no {:?} operator in the current version (please use \"and\", \"or\", \"not\", and \"xor\" instead of C-like operators)", x))
            },
            x if x.chars().next().map(|x| EXPRESSION_SPLIT_CHARS.contains(x)).unwrap_or(false) => {
                return Err(format!("there is no {:?} operator in the current version", x))
            },
            "and" => {
                partial.push(ExprNode::BinOp { op: x, precedence: 6 });
            },
            "or" | "xor" => {
                partial.push(ExprNode::BinOp { op: x, precedence: 7 });
            },
            x => {
                let value = x.parse()?;
                partial.push(ExprNode::StringOrNumber(value));
            },
        }
    }
    if partial.is_empty() {
        return Err(
            "an expression or subexpression must not be empty".to_string()
        );
    }
    // Check for attempts to "do$this"
    for n in 0..partial.len() - 1 {
        if let ExprNode::UnOp { op: "$" } = partial[n + 1] {
            if let ExprNode::StringOrNumber(_) = partial[n] {
                return Err("partial substitution is not allowed (you cannot put a \"$\" in the middle of text)".to_string());
            }
        }
    }
    // Glom up all unary operations
    match partial.last().unwrap() {
        ExprNode::UnOp { op: "$" } => {
            return Err("\"$\" must be immediately followed by a name of a control (\"then\" is not allowed as a control name)".to_string())
        },
        ExprNode::UnOp { op } => {
            return Err(format!("{} makes no sense as the last token of an expression", op))
        },
        _ => (),
    }
    for n in (0..partial.len() - 1).rev() {
        if let ExprNode::UnOp { op } = partial[n] {
            let second = partial.remove(n + 1);
            partial.remove(n);
            match (op, second) {
                ("$", ExprNode::StringOrNumber(StringOrNumber::String(str))) => {
                    partial.insert(n, ExprNode::Subexpression(vec![
                        PredicateOp::PushVar(str)
                    ]));
                },
                ("$", _) => {
                    return Err("\"$\" must be immediately followed by a name of a control (indirection is not allowed in the current version)".to_string())
                },
                ("-", ExprNode::StringOrNumber(StringOrNumber::Number(num))) if num.signum() == 1.0 => {
                    partial.insert(n, ExprNode::StringOrNumber(StringOrNumber::Number(-num)));
                },
                (unop, ExprNode::Subexpression(mut subexpr)) => {
                    subexpr.push(match unop {
                        "not" => PredicateOp::Not,
                        "sin" => PredicateOp::Sin,
                        "cos" => PredicateOp::Cos,
                        "tan" => PredicateOp::Tan,
                        "asin" => PredicateOp::ASin,
                        "acos" => PredicateOp::ACos,
                        "atan" => PredicateOp::ATan,
                        "ln" => PredicateOp::Log,
                        "exp" => PredicateOp::Exp,
                        "floor" => PredicateOp::Floor,
                        "ceil" => PredicateOp::Ceil,
                        "abs" => PredicateOp::Abs,
                        "sign" => PredicateOp::Sign,
                        "-" => PredicateOp::Negate,
                        _ => panic!("Unknown unary op: {:?}", unop),
                    });
                    partial.insert(n, ExprNode::Subexpression(subexpr));
                },
                (unop, _) => {
                    return Err(format!("{:?} must be followed by a subexpression or a control substitution", unop))
                },
                // _ => panic!("internal error: non-exhaustive checking/handling for unary op {:?}", op),
            }
        }
    }
    // and, with that, make sure that we're alternating between binary
    // operations and subexpressions
    for (n, wat) in partial.iter().enumerate() {
        let ok = match wat {
            ExprNode::Subexpression(_) | ExprNode::StringOrNumber(_) => n % 2 == 0,
            ExprNode::BinOp { .. } => n % 2 == 1,
            x => panic!("internal error: {:?} left in partial but should have been glommed", x),
        };
        if !ok {
            return Err("malformed expression".to_string());
        }
    }
    if partial.len() % 2 != 1 {
        return Err("binary operator missing second operand".to_string());
    }
    while partial.len() > 1 {
        debug_assert!(partial.len() % 2 == 1);
        let old_len = partial.len();
        // Find the most precedential binary operator (leftmost)
        let mut best = None;
        for n in (1..partial.len()).step_by(2) {
            let (op, precedence) = match &partial[n] {
                ExprNode::BinOp { op, precedence } => (*op, *precedence),
                _ => unreachable!(),
            };
            let is_best = match best.as_ref() {
                None => true,
                Some((_best_n, _best_op, best_precedence)) => {
                    precedence < *best_precedence
                }
            };
            if is_best {
                best = Some((n, op, precedence))
            }
        }
        let (index, op, _) = best.unwrap();
        let mut it = partial.splice((index - 1)..=(index + 1), None);
        let left = it.next().unwrap();
        let _ = it.next().unwrap();
        let right = it.next().unwrap();
        debug_assert!(it.next().is_none());
        drop(it);
        let op = match op {
            "=" => PredicateOp::Eq,
            "≠" => PredicateOp::NotEq,
            ">" => PredicateOp::Greater,
            "≥" => PredicateOp::GreaterEq,
            "<" => PredicateOp::Lesser,
            "≤" => PredicateOp::LesserEq,
            "and" => PredicateOp::And,
            "or" => PredicateOp::Or,
            "xor" => PredicateOp::Xor,
            "+" => PredicateOp::Add,
            "-" => PredicateOp::Sub,
            "*" => PredicateOp::Mul,
            "/" => PredicateOp::Div,
            "%" => PredicateOp::Rem,
            "//" => PredicateOp::IDiv,
            "^" => PredicateOp::Pow,
            "atan2" => PredicateOp::ATan2,
            "min" => PredicateOp::Min,
            "max" => PredicateOp::Max,
            _ => panic!("internal error: unknown binary operation {:?}", op),
        };
        let mut ops = Vec::new();
        left.push_subexpression(&mut ops);
        right.push_subexpression(&mut ops);
        let is_log = op == PredicateOp::Log;
        ops.push(op);
        if is_log {
            // ln(x) / ln(y) = x log y
            ops.push(PredicateOp::Log);
            ops.push(PredicateOp::Div);
        }
        partial.insert(index - 1, ExprNode::Subexpression(ops));
        debug_assert!(partial.len() < old_len);
    }
    assert!(partial.len() == 1);
    let it = partial.pop().unwrap();
    match it {
        ExprNode::Subexpression(ops) => Ok(ops),
        ExprNode::StringOrNumber(son) => Ok(vec![PredicateOp::PushConst(son)]),
        _ => panic!("internal error: partial not fully consumed! {:?}", it),
    }
}

/// Parses the condition portion of an `if` or `elseif`. Will take everything
/// up to the first `then` as the condition expression, and return everything
/// after it as the rest.
pub(super) fn parse_condition(
    tokens: &[String],
) -> Result<(Vec<PredicateOp>, &[String]), String> {
    let then_pos = match tokens.iter().position(|x| x == "then") {
        Some(x) => x,
        None => {
            if tokens.iter().any(|x| x.ends_with("then")) {
                return Err("\"then\" must be cleanly separated from the condition (try adding a space)".to_string());
            } else {
                return Err("condition must end in a \"then\"".to_string());
            }
        }
    };
    if then_pos == 0 {
        return Err("condition cannot be empty".to_string());
    }
    let rest = &tokens[then_pos + 1..];
    Ok((parse_expression(&tokens[..then_pos])?, rest))
}

/// Parses any expression.
pub(super) fn parse_expression(
    tokens: &[String],
) -> Result<Vec<PredicateOp>, String> {
    let mut pieces = Vec::with_capacity(tokens.len());
    // DinNodes are parsed in "shellish", but for convenience, we want to do
    // some additional splitting around "operators". This means you can't
    // quote-escape operators but this seems like a small price to pay.
    for token in (tokens[..tokens.len()]).iter() {
        let mut rest = token.as_str();
        while let Some((split_pos, _)) = rest.char_indices().find(|(_, ch)| {
            data::EXPRESSION_SPLIT_CHARS.contains(|x| x == *ch)
        }) {
            let before = &rest[..split_pos];
            let during = &rest[split_pos..];
            let split_len = during
                .char_indices()
                .nth(1)
                .map(|x| x.0)
                .unwrap_or(during.len());
            let after = &during[split_len..];
            if !before.is_empty() {
                pieces.push(before);
            }
            pieces.push(&during[..split_len]);
            rest = after;
        }
        if !rest.is_empty() {
            pieces.push(rest);
        }
    }
    // Replace >= and <= and != with their funky equivalents
    if pieces.len() > 1 {
        for n in (0..pieces.len() - 1).rev() {
            if pieces[n + 1] == "=" {
                match pieces[n] {
                    ">" => {
                        pieces.splice(n..=n + 1, ["≥"]);
                    }
                    "<" => {
                        pieces.splice(n..=n + 1, ["≤"]);
                    }
                    "!" => {
                        pieces.splice(n..=n + 1, ["≠"]);
                    }
                    // as a concession to JavaScript, accept any number of
                    // consecutive `=` as being equivalent to one.
                    "=" => {
                        pieces.remove(n + 1);
                    }
                    _ => (),
                }
            } else if pieces[n] == "/" {
                if pieces[n + 1] == "/" {
                    // implement `//` operator
                    pieces.splice(n..=n + 1, ["//"]);
                } else if pieces[n + 1] == "//" {
                    return Err(
                        "too many \"/\"s in a row in expression".to_string()
                    );
                }
            }
        }
    }
    let mut it = pieces.into_iter();
    parse_partial(&mut it, true)
}
