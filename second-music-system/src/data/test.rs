use super::*;

fn show_command_diff(desired: &[Command], seen: &[Command]) {
    for n in 0 .. desired.len().min(seen.len()) {
        if desired[n] == seen[n] {
            eprintln!("\x1B[1m[{}]:\x1B[0m", n);
            eprintln!("      {:?}", desired[n]);
        }
        else {
            eprintln!("\x1B[1;31m[{}]:\x1B[0m", n);
            eprintln!("Want: {:?}", desired[n]);
            eprintln!("Made: {:?}", seen[n]);
        }
    }
    for n in desired.len().min(seen.len()) .. desired.len().max(seen.len()) {
        eprintln!("\x1B[1;31m[{}]:\x1B[0m", n);
        match desired.get(n) {
            Some(x) => eprintln!("Want: {:?}", x),
            None => eprintln!("Want: NOTHING"),
        }
        match seen.get(n) {
            Some(x) => eprintln!("Made: {:?}", x),
            None => eprintln!("Made: NOTHING"),
        }
    }
}

#[test] fn simple_flatten_if() {
    let mut commands = vec![
        Command::PlaySequenceAndWait("drumroll".to_compact_string()),
        Command::If {
            branches: vec![
                (vec![
                    PredicateOp::PushVar("completion".to_compact_string()),
                    PredicateOp::PushConst(StringOrNumber::String("finished".to_compact_string())),
                    PredicateOp::Eq,
                ], vec![Command::StartNode("victory".to_compact_string())]),
                (vec![
                    PredicateOp::PushVar("completion".to_compact_string()),
                    PredicateOp::PushConst(StringOrNumber::String("failed".to_compact_string())),
                    PredicateOp::Eq,
                ], vec![Command::StartNode("defeat".to_compact_string())]),
            ],
            fallback_branch: vec![Command::StartNode("drumroll".to_compact_string())],
        },
    ];
    Command::flatten_commands(&mut commands);
    let correct = vec![
        /*0*/ Command::PlaySequenceAndWait("drumroll".to_compact_string()),
        /*1*/ Command::Goto(vec![
            PredicateOp::PushVar("completion".to_compact_string()),
            PredicateOp::PushConst(StringOrNumber::String("finished".to_compact_string())),
            PredicateOp::Eq,
        ], false, 4),
        /*2*/ Command::StartNode("victory".to_compact_string()),
        /*3*/ Command::Goto(vec![], true, 8),
        /*4*/ Command::Goto(vec![
            PredicateOp::PushVar("completion".to_compact_string()),
            PredicateOp::PushConst(StringOrNumber::String("failed".to_compact_string())),
            PredicateOp::Eq,
        ], false, 7),
        /*5*/ Command::StartNode("defeat".to_compact_string()),
        /*6*/ Command::Goto(vec![], true, 8),
        /*7*/ Command::StartNode("drumroll".to_compact_string()),
    ];
    if commands != correct {
        show_command_diff(&correct, &commands);
        panic!("commands did not come out right");
    }
}

#[test] fn complex_flatten_if() {
    let mut commands = vec![
        Command::PlaySequenceAndWait("drumroll".to_compact_string()),
        Command::If {
            branches: vec![
                (vec![
                    PredicateOp::PushVar("completion".to_compact_string()),
                    PredicateOp::PushConst(StringOrNumber::String("unfinished".to_compact_string())),
                    PredicateOp::NotEq,
                ], vec![
                    Command::If {
                        branches: vec![
                            (vec![
                                PredicateOp::PushVar("completion".to_compact_string()),
                                PredicateOp::PushConst(StringOrNumber::String("finished".to_compact_string())),
                                PredicateOp::Eq,
                            ], vec![Command::StartNode("victory".to_compact_string())]),
                            (vec![
                                PredicateOp::PushVar("completion".to_compact_string()),
                                PredicateOp::PushConst(StringOrNumber::String("failed".to_compact_string())),
                                PredicateOp::Eq,
                            ], vec![Command::StartNode("defeat".to_compact_string())]),
                        ],
                        fallback_branch: vec![],
                    },
                ]),
            ],
            fallback_branch: vec![Command::StartNode("drumroll".to_compact_string())],
        },
    ];
    Command::flatten_commands(&mut commands);
    let correct = vec![
        /*0*/ Command::PlaySequenceAndWait("drumroll".to_compact_string()),
        /*1*/ Command::Goto(vec![
            PredicateOp::PushVar("completion".to_compact_string()),
            PredicateOp::PushConst(StringOrNumber::String("unfinished".to_compact_string())),
            PredicateOp::NotEq,
        ], false, 9),
        /*2*/ Command::Goto(vec![
            PredicateOp::PushVar("completion".to_compact_string()),
            PredicateOp::PushConst(StringOrNumber::String("finished".to_compact_string())),
            PredicateOp::Eq,
        ], false, 5),
        /*3*/ Command::StartNode("victory".to_compact_string()),
        /*4*/ Command::Goto(vec![], true, 8),
        /*5*/ Command::Goto(vec![
            PredicateOp::PushVar("completion".to_compact_string()),
            PredicateOp::PushConst(StringOrNumber::String("failed".to_compact_string())),
            PredicateOp::Eq,
        ], false, 8),
        /*6*/ Command::StartNode("defeat".to_compact_string()),
        /*7*/ Command::Goto(vec![], true, 8),
        /*8*/ Command::Goto(vec![], true, 10),
        /*9*/ Command::StartNode("drumroll".to_compact_string()),
    ];
    if commands != correct {
        show_command_diff(&correct, &commands);
        panic!("commands did not come out right");
    }
}
