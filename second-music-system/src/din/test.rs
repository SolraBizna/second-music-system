use super::*;

macro_rules! end {
    () => {
        ParseItem::EndNode
    };
}
macro_rules! begin {
    ($lineno:expr, $($x:expr),+) => {
        ParseItem::BeginNode(vec![$($x.to_string()),+], $lineno)
    };
}

const TEST_DOCUMENT: &[u8] = br#"formidable azure
    approaching storm
    bringer of death

cold-blooded argent
    unbendable backbone
        treasury of the forsaken kingdom
        patron of the takers
    strengthen thy servant
        the one who listens to thee
            searches for thee
            and is blindly obedient
        shall be answered

amber
    rapture of wilderness
        chaos of maelstroms
            "banner of the righteous"
                essence of blood

crimson
    furious
    color of
        avengers
        prophets
"#;

#[test]
fn new_din_parser() {
    let bytes: &[u8] = &[b'd', b'a'];
    assert_eq!(
        DinParser::new(bytes),
        DinParser {
            rem: bytes,
            lineno: 1,
            indentation_levels: vec![],
            nodes_to_yield: vec![],
        }
    )
}

#[test]
fn iterate_din_parser() {
    let collected: Result<Vec<ParseItem>, _> =
        DinParser::new(TEST_DOCUMENT).collect();
    let desired: &[ParseItem] = &[
        begin!(1, "formidable", "azure"),
        begin!(2, "approaching", "storm"),
        end!(),
        begin!(3, "bringer", "of", "death"),
        end!(),
        end!(),
        begin!(5, "cold-blooded", "argent"),
        begin!(6, "unbendable", "backbone"),
        begin!(7, "treasury", "of", "the", "forsaken", "kingdom"),
        end!(),
        begin!(8, "patron", "of", "the", "takers"),
        end!(),
        end!(),
        begin!(9, "strengthen", "thy", "servant"),
        begin!(10, "the", "one", "who", "listens", "to", "thee"),
        begin!(11, "searches", "for", "thee"),
        end!(),
        begin!(12, "and", "is", "blindly", "obedient"),
        end!(),
        end!(),
        begin!(13, "shall", "be", "answered"),
        end!(),
        end!(),
        end!(),
        begin!(15, "amber"),
        begin!(16, "rapture", "of", "wilderness"),
        begin!(17, "chaos", "of", "maelstroms"),
        begin!(18, "banner of the righteous"),
        begin!(19, "essence", "of", "blood"),
        end!(),
        end!(),
        end!(),
        end!(),
        end!(),
        begin!(21, "crimson"),
        begin!(22, "furious"),
        end!(),
        begin!(23, "color", "of"),
        begin!(24, "avengers"),
        end!(),
        begin!(25, "prophets"),
        end!(),
        end!(),
        end!(),
    ];
    let collected = collected.unwrap();
    if collected[..] != desired[..] {
        let mut lefts = collected.iter();
        let mut rights = desired.iter();
        loop {
            let left = lefts.next();
            let right = rights.next();
            if left.is_none() && right.is_none() {
                break;
            }
            if left == right {
                eprintln!("  {:?}\n  {:?}\n", left, right);
            } else {
                eprintln!("\x1b[31;1m! {:?}\n! {:?}\n\x1b[0m", left, right);
            }
        }
        panic!("Not equal!")
    }
}

#[test]
fn parse_din_nodes() {
    assert_eq!(
        parse_din(std::str::from_utf8(TEST_DOCUMENT).unwrap()).unwrap(),
        &[
            node!(
                1,
                ["formidable", "azure"],
                [
                    node!(2, ["approaching", "storm"]),
                    node!(3, ["bringer", "of", "death"]),
                ]
            ),
            node!(
                5,
                ["cold-blooded", "argent"],
                [
                    node!(
                        6,
                        ["unbendable", "backbone"],
                        [
                            node!(
                                7,
                                [
                                    "treasury", "of", "the", "forsaken",
                                    "kingdom"
                                ]
                            ),
                            node!(8, ["patron", "of", "the", "takers"]),
                        ]
                    ),
                    node!(
                        9,
                        ["strengthen", "thy", "servant"],
                        [
                            node!(
                                10,
                                ["the", "one", "who", "listens", "to", "thee"],
                                [
                                    node!(11, ["searches", "for", "thee"]),
                                    node!(
                                        12,
                                        ["and", "is", "blindly", "obedient"]
                                    ),
                                ]
                            ),
                            node!(13, ["shall", "be", "answered"]),
                        ]
                    ),
                ]
            ),
            node!(
                15,
                ["amber"],
                [node!(
                    16,
                    ["rapture", "of", "wilderness"],
                    [node!(
                        17,
                        ["chaos", "of", "maelstroms"],
                        [node!(
                            18,
                            ["banner of the righteous"],
                            [node!(19, ["essence", "of", "blood"]),]
                        ),]
                    ),]
                ),]
            ),
            node!(
                21,
                ["crimson"],
                [
                    node!(22, ["furious"]),
                    node!(
                        23,
                        ["color", "of"],
                        [node!(24, ["avengers"]), node!(25, ["prophets"]),]
                    ),
                ]
            ),
        ]
    );
}
