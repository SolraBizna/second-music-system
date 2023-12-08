use compact_str::CompactString;

use crate::din::*;

#[test]
fn basics() {
    let node = DinNode {
        items: [
            "play",
            "sound",
            "noman",
            "and",
            "wait",
        ].into_iter().map(str::to_string).collect(),
        children: vec![],
        lineno: 1,
    };
    let mut element_type = None;
    let mut name = None;
    let mut and_wait = false;
    parse_din_node!(node, "play" element_type=("sound"|"sequence") [!"and" name=*] and_wait=["and" "wait"]).unwrap();
    assert_eq!(element_type.as_ref().map(CompactString::as_str), Some("sound"));
    assert_eq!(name.as_ref().map(CompactString::as_str), Some("noman"));
    assert!(and_wait);
}
