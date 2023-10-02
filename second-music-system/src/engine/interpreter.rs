use std::collections::HashMap;

use crate::data::{PredicateOp, StringOrNumber};

use compact_str::CompactString;

macro_rules! op {
    ($stack:ident, |$operand:ident| $expr:expr) => {
        {
            assert!($stack.len() >= 1, "stack underflow");
            let $operand = $stack.pop().unwrap();
            let result = $expr.into();
            $stack.push(result);
        }
    };
    ($stack:ident, |$lhs:ident, $rhs:ident| $expr:expr) => {
        {
            assert!($stack.len() >= 2, "stack underflow");
            let $rhs = $stack.pop().unwrap();
            let $lhs = $stack.pop().unwrap();
            let result = $expr.into();
            $stack.push(result);
        }
    };
}

pub(crate) fn evaluate(flow_controls: &HashMap<CompactString, StringOrNumber>,
    ops: &[PredicateOp]) -> StringOrNumber {
    let mut stack: Vec<StringOrNumber> = Vec::with_capacity(16);
    for op in ops.iter() {
        use PredicateOp::*;
        match op {
            PushVar(x) => stack.push(flow_controls.get(x).cloned().unwrap_or_else(StringOrNumber::default)),
            PushConst(x) => stack.push(x.clone()),
            Eq => op!(stack, |a, b| a == b),
            NotEq => op!(stack, |a, b| a != b),
            Greater => op!(stack, |a, b| a > b),
            GreaterEq => op!(stack, |a, b| a >= b),
            Lesser => op!(stack, |a, b| a < b),
            LesserEq => op!(stack, |a, b| a <= b),
            And => op!(stack, |a, b| a.is_truthy() && b.is_truthy()),
            Or => op!(stack, |a, b| a.is_truthy() || b.is_truthy()),
            Xor => op!(stack, |a, b| a.is_truthy() ^ b.is_truthy()),
            Not => op!(stack, |a| !a.is_truthy()),
            Add => op!(stack, |a, b| a.as_number() + b.as_number()),
            Sub => op!(stack, |a, b| a.as_number() - b.as_number()),
            Mul => op!(stack, |a, b| a.as_number() * b.as_number()),
            Div => op!(stack, |a, b| a.as_number() / b.as_number()),
            Rem => op!(stack, |a, b| a.as_number() % b.as_number()),
            IDiv => op!(stack, |a, b| (a.as_number() / b.as_number()).floor()),
            Pow => op!(stack, |a, b| a.as_number().powf(b.as_number())),
            Sin => op!(stack, |a| a.as_number().sin().to_degrees()),
            Cos => op!(stack, |a| a.as_number().cos().to_degrees()),
            Tan => op!(stack, |a| a.as_number().tan().to_degrees()),
            ASin => op!(stack, |a| a.as_number().asin().to_degrees()),
            ACos => op!(stack, |a| a.as_number().acos().to_degrees()),
            ATan => op!(stack, |a| a.as_number().atan().to_degrees()),
            ATan2 => op!(stack, |a, b| a.as_number().atan2(b.as_number()).to_degrees()),
            Log => op!(stack, |a| a.as_number().ln()),
            Exp => op!(stack, |a| a.as_number().exp()),
            Floor => op!(stack, |a| a.as_number().floor()),
            Ceil => op!(stack, |a| a.as_number().ceil()),
            Min => op!(stack, |a, b| a.as_number().min(b.as_number())),
            Max => op!(stack, |a, b| a.as_number().max(b.as_number())),
            Abs => op!(stack, |a| a.as_number().abs()),
            Sign => op!(stack, |a| a.as_number().signum()),
            Negate => op!(stack, |a| -a.as_number()),
        }
    }
    assert_eq!(stack.len(), 1, "stack left with more than one value???");
    stack.remove(0)
}
