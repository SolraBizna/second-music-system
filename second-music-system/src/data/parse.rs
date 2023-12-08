use std::collections::HashMap;

use super::*;

use din::*;

mod expression;
use expression::{parse_condition, parse_expression};

mod timebase;
use timebase::*;

#[cfg(test)]
mod test;

const SOUND_TIME_KEYWORDS: &[&str] = &["timebase", "start", "end", "length"];

impl Sound {
    /// Parse a `Sound` from a `DinNode`. This `DinNode` might be an "outline
    /// sound", in which case it will be at the top level and it will have its
    /// own name, or it might be an "inline sound", in which case it will have
    /// had a name generated for it. Thus, it does not parse its own items,
    /// only its children.
    fn parse_din_node(
        mut node: DinNode,
        timebases: &TimebaseCollection,
        name: CompactString,
    ) -> Result<Sound, String> {
        let mut timebases = timebases.make_child();
        let mut path = None;
        let mut time_data = HashMap::new();
        let mut offset = None;
        let stream = parse_optional_prefixed_child!(node, "stream")?;
        parse_optional_prefixed_child!(node, "file" path=*)?;
        if let Some(path) = path.as_ref() {
            if path.contains('\0') {
                return Err(format!(
                    "line {}: null characters are not allowed in paths",
                    node.lineno
                ));
            }
        }
        for child in node.consume_designated_children(SOUND_TIME_KEYWORDS) {
            if child.items[0] == "timebase" {
                timebases.parse_timebase_node(&child)?;
            } else {
                let time = timebases.parse_time_node(&child)?;
                time_data.insert(child.items[0].clone(), time);
            }
        }
        if let Some(child) = node.consume_optional_prefixed_child("offset")? {
            if let Some(value) = child.items[1]
                .parse()
                .ok()
                .and_then(|x| PosFloat::new(x).ok())
            {
                offset = Some(value);
            } else {
                return Err(format!(
                    "line {}: that doesn't appear to be a valid number",
                    child.lineno
                ));
            }
        }
        let offset = offset.unwrap_or(PosFloat::ZERO);
        let start = match time_data.get("start") {
            Some(x) => *x + offset,
            None => PosFloat::ZERO,
        };
        let end =
            match (time_data.get("end"), time_data.get("length")) {
                (Some(_), Some(_)) => {
                    return Err(format!(
                        "line {}: only one of \"end\" and \"
                length\" may be specified, not both",
                        node.lineno
                    ))
                }
                (Some(x), None) => *x + offset,
                (None, Some(x)) => start + *x,
                (None, None) => return Err(format!(
                    "line {}: one of \"end\" or \"length\" must be specified",
                    node.lineno
                )),
            };
        node.finish_parsing_children()?;
        // TODO: fade out requires length
        let path = match path {
            Some(path) => path.to_compact_string(),
            None => {
                if let Some(index) = name.find(['\0']) {
                    return Err(format!("Sound {name:?} has a null character in its name at position {index} and no explicit path. If there is no explicit path, the name is used as the path, and the path is not allowed to have null characters in it. Either remove the null character from the name or add an explicit path."));
                }
                name.clone()
            }
        };
        Ok(Sound {
            name,
            path,
            start,
            end,
            stream,
        })
    }
}

impl Sequence {
    /// Parse a `Sequence` from a `DinNode`. This `DinNode` might be an
    /// "outline sequence", in which case it will be at the top level and it
    /// will have its own name, or it might be an "inline sequence", in which
    /// case it will have had a name generated for it. Thus, it does not parse
    /// its own items, only its children.
    fn parse_din_node(
        mut node: DinNode,
        timebases: &TimebaseCollection,
        name: CompactString,
    ) -> Result<Sequence, String> {
        let mut timebases = timebases.make_child();
        let length = node
            .consume_required_prefixed_child("length")
            .and_then(|child| timebases.parse_time_node(&child))?;
        let mut elements = Vec::new();
        for child in node.consume_designated_children(&["play", "timebase"]) {
            match child.items[0].as_str() {
                "play" => {
                    let (start, element) = SequenceElement::parse_din_node(
                        child, &timebases, &name,
                    )?;
                    elements.push((start, element));
                }
                "timebase" => {
                    timebases.parse_timebase_node(&child)?;
                }
                _ => unreachable!(),
            }
        }
        node.finish_parsing_children()?;
        elements.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(Sequence {
            name,
            length,
            elements,
        })
    }
}

const SOUND_ELEMENT_TIME_KEYWORDS: &[&str] =
    &["timebase", "at", "for", "until", "fade_in", "fade_out"];

const SEQUENCE_ELEMENT_TIME_KEYWORDS: &[&str] = &["timebase", "at"];

impl SequenceElement {
    fn parse_din_node(
        mut node: DinNode,
        timebases: &TimebaseCollection,
        sequence_name: &str,
    ) -> Result<(PosFloat, SequenceElement), String> {
        let mut element_type = None;
        let mut name = None;
        parse_din_node!(node, "play" element_type=("sound"|"sequence") [name=*])?;
        let element_type = element_type.unwrap();
        let time_keywords = match element_type.as_str() {
            "sound" => SOUND_ELEMENT_TIME_KEYWORDS,
            "sequence" => SEQUENCE_ELEMENT_TIME_KEYWORDS,
            _ => unreachable!(),
        };
        let mut timebases = timebases.make_child();
        let mut data = HashMap::new();
        let mut channel = None;
        if element_type == "sound" {
            parse_optional_prefixed_child!(node, "channel" channel=*)?;
        }
        for child in node.consume_designated_children(time_keywords) {
            if child.items[0] == "timebase" {
                timebases.parse_timebase_node(&child)?;
            } else if data.contains_key(child.items[0].as_str()) {
                return Err(format!(
                    "line {}: only one {:?} parameter allowed",
                    child.lineno, child.items[0]
                ));
            } else {
                let time = timebases.parse_time_node(&child)?;
                data.insert(child.items[0].clone(), time);
            }
        }
        let anonymous;
        let name = match name {
            None => {
                anonymous = true;
                format!("{sequence_name}[{}]", node.lineno).to_compact_string()
            }
            Some(x) => {
                anonymous = false;
                x.to_compact_string()
            }
        };
        if anonymous != node.children.is_empty() {
            return Err(format!("line {}: \"play\" must either specify the name of the {element_type} to be played, or provide an inline definition for it (not both nor neither!)", node.lineno));
        }
        let channel = channel
            .as_ref()
            .map(CompactString::as_str)
            .unwrap_or("main")
            .to_compact_string();
        let start = match data.get("at") {
            Some(x) => *x,
            None => PosFloat::ZERO,
        };
        let fade_in = match data.get("fade_in") {
            Some(x) => *x,
            None => PosFloat::ZERO,
        };
        let length = match (data.get("for"), data.get("until")) {
            (Some(_), Some(_)) => {
                return Err(format!("line {}: only one of \"for\" and \"until\" may be specified, not both", node.lineno))
            },
            (None, None) => {
                None
            },
            (Some(length), None) => {
                Some(*length)
            },
            (None, Some(end)) => {
                Some(end.saturating_sub(start))
            },
        };
        let (length, fade_out) = match data.get("fade_out") {
            Some(fade_out) => {
                (length.map(|x| x.saturating_sub(*fade_out)), *fade_out)
            }
            None => (length, PosFloat::ZERO),
        };
        node.finish_parsing_children()?;
        match element_type.as_str() {
            "sound" => Ok((
                start,
                SequenceElement::PlaySound {
                    sound: name.to_compact_string(),
                    channel,
                    fade_in,
                    length,
                    fade_out,
                },
            )),
            "sequence" => Ok((
                start,
                SequenceElement::PlaySequence {
                    sequence: name.to_compact_string(),
                },
            )),
            _ => unreachable!(),
        }
    }
}

fn parse_flow_command_tokens(
    tokens: &[String],
    timebases: &TimebaseCollection,
) -> Result<Option<Command>, String> {
    if tokens.is_empty() {
        return Ok(None);
    }
    match tokens[0].as_str() {
        "done" => {
            if tokens.len() != 1 {
                return Err("nothing is allowed after \"done\"".to_string());
            }
            Ok(Some(Command::Done))
        }
        "wait" => {
            let how_long = timebases.parse_time(tokens)?;
            Ok(Some(Command::Wait(how_long)))
        }
        "play" => {
            let token = tokens.get(1).map(String::as_str);
            match token {
                Some("sequence") | Some("sound") => (),
                _ => return Err("next element after \"play\" must be \"sequence\" or \"sound\"".to_string())
            }
            let target = match tokens.get(2) {
                Some(x) => x,
                None => return Err(format!("next element after \"{}\" must be the name of the {} to play", token.unwrap(), token.unwrap())),
            }.to_compact_string();
            let and_wait = if tokens.len() == 3 {
                false
            } else if tokens.len() == 5
                && tokens[3] == "and"
                && tokens[4] == "wait"
            {
                true
            } else {
                return Err("the only thing allowed after the name of the sequence or sound to play is the elements \"and wait\" (do you need quotation marks?)".to_string());
            };
            Ok(Some(match (token.unwrap(), and_wait) {
                ("sequence", false) => Command::PlaySequence(target),
                ("sequence", true) => Command::PlaySequenceAndWait(target),
                ("sound", false) => Command::PlaySound(target),
                ("sound", true) => Command::PlaySoundAndWait(target),
                _ => unreachable!(),
            }))
        }
        "start" | "restart" | "stop" => {
            match tokens.get(1).map(String::as_str) {
                Some("node") => {
                    let target = match tokens.get(2) {
                        Some(x) => x,
                        None => return Err(format!("next element after \"node\" must be the name of the node to {}", tokens[0])),
                    }.to_compact_string();
                    if tokens.len() != 3 {
                        return Err("nothing is allowed after the node name (do you need quotation marks?)".to_string())
                    };
                    match tokens[0].as_str() {
                        "start" => Ok(Some(Command::StartNode(target))),
                        "restart" => Ok(Some(Command::RestartNode(target))),
                        "stop" => {
                            Err("stop is not allowed because it will sound bad (if you really want an abrupt cutoff, try `fade NodeName over 0`)".to_string())
                        }
                        _ => unreachable!(),
                    }
                },
                Some("starting") => {
                    if tokens.get(0).map(String::as_str) != Some("restart") {
                        return Err(format!("next element after \"restart\" must be \"node\" or \"starting\""))
                    }
                    if tokens.get(2).map(String::as_str) != Some("node") {
                        return Err(format!("next element after \"starting\" must be \"node\""))
                    }
                    if tokens.len() != 3 {
                        return Err(format!("nothing is allowed after \"restart starting node\""))
                    }
                    Ok(Some(Command::RestartFlow))
                },
                Some(x) => Err(format!("invalid element \"{}\" next element after {:?} must be \"node\" or \"starting\"", x, tokens[0])),
                None => Err(format!("\"{:?}\" must be followed by \"node\" or \"starting\"", tokens[0]))
            }
        }
        "fade" => {
            if tokens.get(1).map(String::as_str) != Some("node") {
                return Err(
                    "next element after \"fade\" must be \"node\"".to_string()
                );
            }
            let target = match tokens.get(2) {
                Some(x) => x,
                None => return Err("next element after \"node\" must be the name of the node to fade".to_string()),
            }.to_compact_string();
            if tokens.get(3).map(String::as_str) != Some("over") {
                return Err("next element after node name must be \"over\""
                    .to_string());
            }
            let length = timebases.parse_time(&tokens[3..])?;
            Ok(Some(Command::FadeNodeOut(target, length)))
        }
        "set" => {
            let target = match tokens.get(1) {
                Some(x) => x,
                None => return Err("next element after \"set\" must be the name of the flow control to set".to_string()),
            }.to_compact_string();
            if tokens.get(2).map(String::as_str) != Some("to") {
                return Err(
                    "next element after node name must be \"to\"".to_string()
                );
            }
            Ok(Some(Command::Set(target, parse_expression(&tokens[3..])?)))
        }
        "if" => {
            // If we get here, we're an inline if. No children.
            let (condition, rest) = parse_condition(&tokens[1..])?;
            let command = match parse_flow_command_tokens(rest, timebases)? {
                Some(x) => x,
                None => {
                    return Err(
                        "there needs to be a command after the \"then\""
                            .to_string(),
                    )
                }
            };
            Ok(Some(Command::If {
                branches: vec![(condition, vec![command])],
                fallback_branch: vec![],
            }))
        }
        "else" => Err(
            "else is not allowed here (try breaking it onto its own line)"
                .to_string(),
        ),
        "elseif" => Err(
            "elseif is not allowed here (try breaking it onto its own line)"
                .to_string(),
        ),
        _ => Ok(None),
    }
}

fn parse_node_child_code(
    node: &mut DinNode,
    timebases: &TimebaseCollection,
) -> Result<Vec<Command>, String> {
    let mut timebases = timebases.make_child();
    let mut commands = vec![];
    for mut child in node.consume_children() {
        debug_assert!(!child.items.is_empty());
        if child.items[0] == "timebase" {
            timebases.parse_timebase_node(&child)?;
        } else if child.items[0] == "node" {
            return Err(format!(
                "line {}: nodes cannot be nested",
                child.lineno
            ));
        } else if let Some(command) = parse_flow_command_node(
            &mut child,
            &timebases,
            commands.last_mut(),
        )? {
            if let Some(command) = command {
                // it was a command to add
                commands.push(command);
            } else {
                // it was an `else` or `elseif`, and we have nothing to do
            }
        } else {
            return Err(format!(
                "line {}: unknown node element {:?}",
                child.lineno, child.items[0]
            ));
        }
    }
    Ok(commands)
}

fn parse_if_body(
    node: &mut DinNode,
    rest: &[String],
    timebases: &TimebaseCollection,
) -> Result<Vec<Command>, String> {
    if !rest.is_empty() {
        if !node.children.is_empty() {
            return Err(format!("{} can have an inline body (right after the \"then\") or children (indented lines afterward) but not both", node.items[0]));
        }
        let command = match parse_flow_command_tokens(rest, timebases)? {
            Some(x) => x,
            None => return Err("unknown command after \"then\"".to_string()),
        };
        Ok(vec![command])
    } else {
        // not an error if no children
        parse_node_child_code(node, timebases)
    }
}

/// Tentatively parse a `DinNode` that corresponds to a single command within
/// a `Node`.
///
/// - `Err(x)`: A parse error
/// - `Ok(None)`: Unknown command
/// - `Ok(Some(None))`: `else` or `else if` that successfully got folded into
///   a preceding `If`
/// - `Ok(Some(Some(command)))`: A command you must now append to the list
fn parse_flow_command_node(
    node: &mut DinNode,
    timebases: &TimebaseCollection,
    last_command: Option<&mut Command>,
) -> Result<Option<Option<Command>>, String> {
    if node.items[0] == "if" {
        // If we get here, we might either be an inline if or an expanded one.
        let mut items = vec![];
        std::mem::swap(&mut items, &mut node.items);
        let (condition, rest) = parse_condition(&items[1..])?;
        let commands = parse_if_body(node, rest, timebases)?;
        Ok(Some(Some(Command::If {
            branches: vec![(condition, commands)],
            fallback_branch: vec![],
        })))
    } else if node.items[0] == "else" {
        let (last_branches, last_fallback_branch) = match last_command {
            Some(Command::If { branches, fallback_branch }) => (branches, fallback_branch),
            _ => {
                return Err(format!("line {}: \"else\" without matching \"if\" (check indentation)", node.lineno))
            },
        };
        if node.items.get(1).map(String::as_str) == Some("if") {
            // We are an else if
            let mut items = vec![];
            std::mem::swap(&mut items, &mut node.items);
            let (condition, rest) = parse_condition(&items[2..])?;
            let commands = parse_if_body(node, rest, timebases)?;
            last_branches.push((condition, commands));
        } else {
            // We are an else
            let commands = parse_if_body(node, &[], timebases)?;
            if !last_fallback_branch.is_empty() {
                return Err(format!("line {}: only one \"else\" is allowed for a given \"if\" chain (check indentation)", node.lineno));
            }
            if commands.is_empty() {
                return Err(format!("line {}: \"else\" must contain at least oen command (check indentation or delete this line)", node.lineno));
            }
            *last_fallback_branch = commands;
        }
        Ok(Some(None))
    } else if node.items[0] == "elseif" {
        let last_branches = match last_command {
            Some(Command::If { branches, .. }) => branches,
            _ => {
                return Err(format!("line {}: \"elseif\" without matching \"if\" (check indentation)", node.lineno))
            },
        };
        let mut items = vec![];
        std::mem::swap(&mut items, &mut node.items);
        let (condition, rest) = parse_condition(&items[1..])?;
        let commands = parse_if_body(node, rest, timebases)?;
        last_branches.push((condition, commands));
        Ok(Some(None))
    } else {
        let parsed = parse_flow_command_tokens(&node.items, timebases);
        match parsed {
            Ok(Some(x)) => {
                if !node.children.is_empty() {
                    return Err(format!(
                        "line {}: this element must have no children",
                        node.lineno
                    ));
                }
                Ok(Some(Some(x)))
            }
            Ok(None) => Ok(None),
            Err(x) => Err(format!("line {}: {}", node.lineno, x)),
        }
    }
}

impl Node {
    fn parse_node(
        mut din_node: DinNode,
        timebases: &TimebaseCollection,
    ) -> Result<Node, String> {
        assert_eq!(din_node.items[0], "node");
        if din_node.items.len() != 2 {
            return Err(format!(
                "line {}: node element must have a name",
                din_node.lineno
            ));
        }
        let name = din_node.items[1].to_compact_string();
        let commands = parse_node_child_code(&mut din_node, timebases)?;
        din_node.finish_parsing_children()?;
        Ok(Node {
            name: Some(name),
            commands,
        })
    }
}

impl Flow {
    fn parse_din_node(
        mut node: DinNode,
        timebases: &TimebaseCollection,
        flows: &HashMap<CompactString, Arc<Flow>>,
    ) -> Result<Flow, String> {
        let mut name = None;
        parse_din_node!(node, "flow" name=*)?;
        let name = name.unwrap().to_compact_string();
        let mut timebases = timebases.make_child();
        let mut nodes = flows
            .get(&name)
            .map(|x| x.nodes.clone())
            .unwrap_or_else(|| HashMap::new());
        let mut start_node = Node::new();
        for mut child in node.consume_children() {
            debug_assert!(!child.items.is_empty());
            if child.items[0] == "timebase" {
                timebases.parse_timebase_node(&child)?;
            } else if child.items[0] == "node" {
                let mut node = Node::parse_node(child, &timebases)?;
                Command::flatten_commands(&mut node.commands);
                nodes.insert(node.name.clone().unwrap(), Arc::new(node));
            } else if let Some(command) = parse_flow_command_node(
                &mut child,
                &timebases,
                start_node.commands.last_mut(),
            )? {
                if let Some(command) = command {
                    // it was a command to add
                    start_node.commands.push(command);
                } else {
                    // it was an `else` or `elseif`, and we have nothing to do
                }
            } else {
                return Err(format!(
                    "line {}: unknown flow element {:?}",
                    child.lineno, child.items[0]
                ));
            }
        }
        Command::flatten_commands(&mut start_node.commands);
        let new_flow = Flow {
            name,
            start_node: Arc::new(start_node),
            nodes,
        };
        Ok(new_flow)
    }
}

impl Soundtrack {
    pub fn parse_source(mut self, source: &str) -> Result<Soundtrack, String> {
        let document = parse_din(source)?;
        let mut timebases = TimebaseCollection::new();
        for node in document.into_iter() {
            assert!(!node.items.is_empty());
            match node.items[0].as_str() {
                "timebase" => timebases.parse_timebase_node(&node)?,
                "sound" => {
                    let mut name = None;
                    parse_din_node!(node, "sound" name=*)?;
                    let name = name.unwrap().to_compact_string();
                    let sound = Sound::parse_din_node(node, &timebases, name.clone())?;
                    debug_assert_eq!(sound.name, name);
                    self.sounds.insert(name, Arc::new(sound));
                },
                "sequence" => {
                    let mut name = None;
                    parse_din_node!(node, "sequence" name=*)?;
                    let name = name.unwrap().to_compact_string();
                    let sequence = Sequence::parse_din_node(node, &timebases, name.clone())?;
                    debug_assert_eq!(sequence.name, name);
                    self.sequences.insert(name, Arc::new(sequence));
                },
                "flow" => {
                    let flow = Flow::parse_din_node(node, &timebases, &self.flows)?;
                    self.flows.insert(flow.name.clone(), Arc::new(flow));
                },
                "region" => return Err(format!("line {}: regions may only exist inside sequences (check indentation)", node.lineno)),
                "node" => return Err(format!("line {}: nodes may only exist inside flows (check indentation)", node.lineno)),
                x => return Err(format!("line {}: unknown top-level element {:?}", node.lineno, x)),
            }
        }
        Ok(self)
    }
}

impl Command {
    pub fn flatten_commands(commands: &mut Vec<Command>) {
        let mut n = 0;
        while n < commands.len() {
            if let Command::If { .. } = &commands[n] {
                let eye_eff = commands.remove(n);
                let (branches, fallback_branch) = if let Command::If {
                    branches,
                    fallback_branch,
                } = eye_eff
                {
                    (branches, fallback_branch)
                } else {
                    unreachable!()
                };
                // Well that was ugly.
                Command::insert_flattened_if(
                    commands,
                    n,
                    branches,
                    fallback_branch,
                );
            } else {
                n += 1;
            }
        }
    }
    /// Performs one level of flattening. You'll still need to run the
    /// steamroller over the commands we insert.
    fn insert_flattened_if(
        commands: &mut Vec<Command>,
        insertion_point: usize,
        branches: Vec<(Vec<PredicateOp>, Vec<Command>)>,
        mut fallback_branch: Vec<Command>,
    ) {
        #[allow(clippy::identity_op)]
        let buffer_size = 0
            // Two gotos per branch
            + branches.len() * 2
            // One command per... command
            + branches.iter().fold(0, |a,x| a + x.1.len())
            + fallback_branch.len();
        let mut to_insert = Vec::with_capacity(buffer_size);
        let mut exit_goto_positions = Vec::with_capacity(branches.len());
        for (predicate, mut subcommands) in branches.into_iter() {
            let conditional_goto_position = to_insert.len();
            to_insert.push(Command::Placeholder);
            for subcommand in subcommands.iter_mut() {
                if let Command::Goto(_, _, pos) = subcommand {
                    *pos += insertion_point + to_insert.len();
                }
            }
            to_insert.append(&mut subcommands);
            exit_goto_positions.push(to_insert.len());
            to_insert.push(Command::Placeholder);
            to_insert[conditional_goto_position] = Command::Goto(
                predicate,
                false,
                to_insert.len() + insertion_point,
            );
        }
        for subcommand in fallback_branch.iter_mut() {
            if let Command::Goto(_, _, pos) = subcommand {
                *pos += insertion_point + to_insert.len();
            }
        }
        to_insert.append(&mut fallback_branch);
        let exit_position = to_insert.len() + insertion_point;
        for pos in exit_goto_positions.into_iter() {
            to_insert[pos] = Command::Goto(vec![], true, exit_position);
        }
        for command in commands.iter_mut() {
            if let Command::Goto(_, _, target) = command {
                if *target > insertion_point {
                    *target = *target + to_insert.len() - 1;
                }
            }
        }
        commands.splice(insertion_point..insertion_point, to_insert);
    }
}
