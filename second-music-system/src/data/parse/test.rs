use super::*;

#[test] fn timebase_parse() {
    assert_eq!(
        Timebase::parse_source(&vec![
            "@4".to_string(),
            "120/m".to_string(),
            "32".to_string(),
        ]),
        Ok(Timebase {
            stages: vec![
                TimebaseStage { one_based: true, multiplier: 2.0 },
                TimebaseStage { one_based: false, multiplier: 0.5 },
                TimebaseStage { one_based: false, multiplier: 1.0 / 64.0 }
            ]
        })
    );
}
#[test] fn new_sound_parse() {
    let node = node!(1, ["sound", "test1.mp3"], [
        node!(2, ["length", "32"]),
    ]);
    let timebases = TimebaseCollection::new();
    assert_eq!(
        Sound::parse_din_node(&node, &timebases).unwrap(),
        Sound {
            name: "test1.mp3".to_string(),
            path: "test1.mp3".to_string(),
            start: 0.0,
            end: 32.0,
            stream: false,
        }
    );
}
#[test] fn sound_parse() {
    let soundtrack = Soundtrack::from_source(r#"
sound test1.mp3
    length 32
    "#).unwrap();
    assert_eq!(soundtrack.sounds.len(), 1);
    assert_eq!(**soundtrack.sounds.get("test1.mp3").unwrap(),
    Sound {
        name: "test1.mp3".to_string(),
        path: "test1.mp3".to_string(),
        start: 0.0,
        end: 32.0,
        stream: false,
    });
    assert_eq!(soundtrack.sequences.len(), 0);
    assert_eq!(soundtrack.flows.len(), 0);
}

#[test]
#[should_panic]
fn sound_with_null_implicit_path_parse() {
    let node = node!(1, ["sound", "test\0.mp3"], [
        node!(2, ["length", "32"]),
    ]);
    let timebases = TimebaseCollection::new();
    Sound::parse_din_node(&node, &timebases).unwrap();
}
#[test]
#[should_panic]
fn sound_with_null_path_parse() {
    let node = node!(1, ["sound", "test"], [
        node!(2, ["file", "test\0.mp3"]),
        node!(2, ["length", "32"]),
    ]);
    let timebases = TimebaseCollection::new();
    Sound::parse_din_node(&node, &timebases).unwrap();
}
#[test]
fn sound_with_null_name_explicit_nonnull_path_parse() {
    let node = node!(1, ["sound", "test\0"], [
        node!(2, ["file", "test.mp3"]),
        node!(2, ["length", "32"]),
    ]);
    let timebases = TimebaseCollection::new();
    Sound::parse_din_node(&node, &timebases).unwrap();
}

#[test] fn new_sequence_parse() {
    let node = node!(1, ["sequence", "test1"], [
        node!(2, ["length", "32"]),
    ]);
    let timebases = TimebaseCollection::new();
    assert_eq!(
        Sequence::parse_din_node(&node, &timebases).unwrap(),
        Sequence {
            name: "test1".to_string(),
            length: 32.0,
            elements: vec![],
        }
    );
}
#[test] fn sequence_parse() {
    let soundtrack = Soundtrack::from_source(r#"
sequence test1
    length 32
    "#).unwrap();
    assert_eq!(soundtrack.sounds.len(), 0);
    assert_eq!(soundtrack.sequences.len(), 1);
    assert_eq!(soundtrack.flows.len(), 0);
}
#[test] fn new_sequence_element_parse() {
    let node_one = node!(1, ["sequence", "test1"], [
        node!(2, ["length", "32"]),
    ]);
    let node_two = node!(1, ["sequence", "test2"], [
        node!(2, ["length", "64"]),
        node!(3, ["play", "sequence", "test1"], [
            node!(4, ["at", "32"]),
        ]),
    ]);
    let timebases = TimebaseCollection::new();
    let sequence_one = Sequence::parse_din_node(&node_one, &timebases).unwrap();
    let sequence_two = Sequence::parse_din_node(&node_two, &timebases).unwrap();
    assert_eq!(
        sequence_one,
        Sequence {
            name: "test1".to_string(),
            length: 32.0,
            elements: vec![],
        }
    );
    assert_eq!(
        sequence_two,
        Sequence {
            name: "test2".to_string(),
            length: 64.0,
            elements: vec![
                (32.0, SequenceElement::PlaySequence { 
                    sequence: "test1".to_string() 
                }),
            ],
        }
    );
}
#[test]
fn sequence_element_parse() {
    let soundtrack = Soundtrack::from_source(r#"
sequence test1
    length 32
sequence test2
    length 64
    play sequence test1
        at 32
"#).unwrap();
    assert_eq!(soundtrack.sounds.len(), 0);
    assert_eq!(soundtrack.sequences.len(), 2);
    assert_eq!(**soundtrack.sequences.get("test1").unwrap(),
        Sequence {
            name: "test1".to_string(),
            length: 32.0,
            elements: vec![],
        });
    assert_eq!(**soundtrack.sequences.get("test2").unwrap(),
    Sequence {
        name: "test2".to_string(),
        length: 64.0,
        elements: vec![(32.0, SequenceElement::PlaySequence { sequence: "test1".to_string() })],
    });
    assert_eq!(soundtrack.flows.len(), 0);
}
#[test]
fn invalid_sequence_element_parse() {
    let invalid_parameters = [
        r#"length"#,
        r#"for"#, 
        r#"until"#, 
        r#"fade_in"#, 
        r#"fade_out"#,
    ];
    for parameter in invalid_parameters {
        let soundtrack = Soundtrack::from_source(&format!(r#"
sequence test1
    length 32
sequence test2
    length 64
    play sequence test1
        at 32
        {} 32
"#, parameter));
        // TODO(nemo): extract error strings from parse into their own module
        assert_eq!(soundtrack.err(), Some(format!("line 8: unknown element parameter \"{}\"", parameter)));
    }
}
#[test]
fn flow_empty_parse() {
    let soundtrack = Soundtrack::from_source(r#"
flow test_flow1
    "#).unwrap();
    assert_eq!(soundtrack.sounds.len(), 0);
    assert_eq!(soundtrack.sequences.len(), 0);
    assert_eq!(soundtrack.flows.len(), 1);
    let start_node = Arc::new(Node::new());
    let nodes = HashMap::new();
    assert_eq!(**soundtrack.flows.get("test_flow1").unwrap(), 
        Flow {
            name: "test_flow1".to_string(),
            start_node,
            nodes,
        });
}
#[test]
#[should_panic]
fn flow_no_timebase_parse() {
    Soundtrack::from_source(r#"
sequence test_sequence1
  length 8.0.0
flow test_flow1
  node test_node1
    play sequence test_sequence1
    "#).unwrap();
}
#[test] fn flow_no_trigger_parse() {
    let soundtrack = Soundtrack::from_source(r#"
timebase beat @4 120/m 256
sequence test_sequence1
  timebase beat
  length 8.0.0
flow test_flow1
  node test_node1
    play sequence test_sequence1
    "#).unwrap();
    assert_eq!(soundtrack.sounds.len(), 0);
    assert_eq!(soundtrack.sequences.len(), 1);
    assert_eq!(soundtrack.flows.len(), 1);
    let start_node = Arc::new(Node::new());
    let mut nodes = HashMap::new();
    nodes.insert("test_node1".to_string(), 
        Arc::new(Node { 
            name: Some("test_node1".to_string()), 
            commands: vec![Command::PlaySequence("test_sequence1".to_string())] 
        }));
    assert_eq!(**soundtrack.flows.get("test_flow1").unwrap(), 
        Flow {
            name: "test_flow1".to_string(),
            start_node,
            nodes,
        });
}
#[test] fn fade_out_parse() {
    let node = node!(1, ["sequence", "test1"], [
        node!(2, ["length", "32"]),
        node!(3, ["play", "sound", "test_sound"], [
            node!(4, ["at", "0"]),
            node!(5, ["for", "16"]),
            node!(6, ["fade_out", "4"]),
        ]),
    ]);
    let timebases = TimebaseCollection::new();
    assert_eq!(
        Sequence::parse_din_node(&node, &timebases).unwrap(),
        Sequence {
            name: "test1".to_string(),
            length: 32.0,
            elements: vec![
                (0.0, SequenceElement::PlaySound { 
                    sound: format!("test_sound"), 
                    channel: format!("main"), 
                    fade_in: 0.0, 
                    length: Some(12.0), 
                    fade_out: 4.0,
                })
            ],
        }
    );
}

// TODO: condition styles to test:
//
// if foo then
//   bar
// elseif foo then
//   bar
// else if foo then
//   bar
// else
//   baz
//
// if foo then bar
// elseif foo then bar
// else if foo then bar
// else baz
//

#[test] #[should_panic] fn missingthen() {
    let toks = vec!["dennis".to_string()];
    parse_condition(&toks).unwrap();

}

#[test] fn expression_parsing() {
    let mut goods: Vec<String> = vec![
        "- 5",
        "-5",
        "-$foo",
        "- $foo",
    ].into_iter().map(String::from).collect();
    let mut bads: Vec<String> = [
        // empty
        "",
        // dangling paren
        ")",
        "( foo ) )",
        "(foo))",
        // unbalanced paren
        "(",
        // bad cases of \ (defeats our auto generation)
        "\\\\",
        "\\\\ b",
        "a \\\\",
        "a \\\\ b",
        // bad cases of # (defeats our auto generation)
        "\\#",
        "\\# b",
        "a \\#",
        "a \\# b",
        // bad cases of $
        "$",
        "test $",
        "test $ toast",
        "$ test toast",
        "$ 7",
        "$ (thing)",
        // too many slashes
        "a / / / b",
        // malformed expression
        "2 > 3 4",
        "4 5 = 6",
        // forgot the $
        "-foo",
        "- foo",
    ].into_iter().map(String::from).collect();
    const EXCLUDED_AUTO_BADS: &str = "#$<>=≤≥≠\\/-+^%*";
    for ch in EXPRESSION_SPLIT_CHARS.chars() {
        if !EXCLUDED_AUTO_BADS.contains(ch) {
            bads.push(format!("{}", ch));
            bads.push(format!("{} b", ch));
            bads.push(format!("a {}", ch));
            bads.push(format!("a {} b", ch));
        }
    }
    const BINARY_OPS: &[&str] = &[
        "==", "!=", ">", ">=", "<", "<=", "and", "or", "xor",
        "+", "-", "*", "/", "//", "%", "^", "atan2", "min", "max",
    ];
    for op in BINARY_OPS.iter() {
        bads.push(format!("{}", op));
        bads.push(format!("{} b", op));
        bads.push(format!("a {}", op));
        if *op != "/" && *op != "==" {
            bads.push(format!("a {} {} b", op, op));
        }
        goods.push(format!("a {} b", op));
    }
    let mut failed_bads = 0;
    for bad in bads.iter() {
        let toks = shellish_parse::parse(bad, shellish_parse::ParseOptions::new()).unwrap();
        match parse_expression(&toks) {
            Ok(x) => {
                eprintln!("\x1b[31;1mSHOULD NOT HAVE PARSED! {:?} parsed as {:?}\x1b[0m", bad, x);
                failed_bads += 1;
            },
            Err(x) => {
                eprintln!("OK: {:?} -> {}", bad, x);
            },
        }
    }
    if failed_bads > 0 {
        panic!("Some lines that should not have parsed parsed! (See output)");
    }
    let mut failed_goods = 0;
    for good in goods.iter() {
        let toks = shellish_parse::parse(good, shellish_parse::ParseOptions::new()).unwrap();
        match parse_expression(&toks) {
            Err(x) => {
                eprintln!("\x1b[31;1mSHOULD HAVE PARSED! {:?} -> {}\x1b[0m", good, x);
                failed_goods += 1;
            },
            Ok(x) => {
                eprintln!("OK: {:?} -> {:?}", good, x);
            },
        }
    }
    if failed_goods > 0 {
        panic!("Some lines that should have parsed did not! (See output)");
    }
}