use super::*;

use function_name::named;

macro_rules! target {
    ($target:expr, $function_name:expr) => {
        if $target.is_null() {
            panic!("{}: target cannot be NULL!", $function_name);
        }
        else {
            unsafe { $target.as_mut().unwrap() }
        }
    }
}

macro_rules! fade_type {
    ($fade_type:expr, $function_name:expr) => {
        match fade_type_from_int($fade_type) {
            Some(x) => x,
            None => panic!("{}: fade_type must be a valid SMS_FADE_TYPE_* constant!", $function_name),
        }
    }
}

macro_rules! implement_commands {
($c_target:ty, $rust_target:ty) => { paste::paste!{

#[no_mangle] #[named]
extern "C" fn [<$c_target _ begin_transaction>](
    target: *mut $rust_target,
    length: size_t,
) -> *mut Transaction<'static, dyn EngineCommandIssuer> {
    let target = target!(target, function_name!());
    let length = if length == 0 { None } else { Some(length as usize) };
    // yikes!!
    let issuer: &mut dyn EngineCommandIssuer = target;
    Box::into_raw(Box::new(issuer.begin_transaction(length)))
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ replace_soundtrack>](
    target: *mut $rust_target,
    new_soundtrack: *mut Soundtrack,
) {
    let target = target!(target, function_name!());
    let new_soundtrack = *unsafe { Box::from_raw(new_soundtrack) };
    target.replace_soundtrack(new_soundtrack);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ precache>](
    target: *mut $rust_target,
    flow_name: *const c_char,
    flow_name_len: size_t,
) {
    let target = target!(target, function_name!());
    let flow_name = input(flow_name, flow_name_len).unwrap();
    target.precache(flow_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ precache_cstr>](
    target: *mut $rust_target,
    flow_name: *const c_char,
) {
    let target = target!(target, function_name!());
    let flow_name = input_cstr(flow_name).unwrap();
    target.precache(flow_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ unprecache>](
    target: *mut $rust_target,
    flow_name: *const c_char,
    flow_name_len: size_t,
) {
    let target = target!(target, function_name!());
    let flow_name = input(flow_name, flow_name_len).unwrap();
    target.unprecache(flow_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ unprecache_cstr>](
    target: *mut $rust_target,
    flow_name: *const c_char,
) {
    let target = target!(target, function_name!());
    let flow_name = input_cstr(flow_name).unwrap();
    target.unprecache(flow_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ unprecache_all>](
    target: *mut $rust_target,
) {
    let target = target!(target, function_name!());
    target.unprecache_all();
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ set_flow_control_to_number>](
    target: *mut $rust_target,
    control_name: *const c_char,
    control_name_len: size_t,
    new_value: f32
) {
    let target = target!(target, function_name!());
    let control_name = input(control_name, control_name_len).unwrap();
    target.set_flow_control(control_name, StringOrNumber::Number(new_value));
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ set_flow_control_to_string>](
    target: *mut $rust_target,
    control_name: *const c_char,
    control_name_len: size_t,
    new_value: *const c_char,
    new_value_len: size_t,
) {
    let target = target!(target, function_name!());
    let control_name = input(control_name, control_name_len).unwrap();
    let new_value = input(new_value, new_value_len).unwrap();
    target.set_flow_control(control_name, StringOrNumber::String(new_value));
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ clear_flow_control>](
    target: *mut $rust_target,
    control_name: *const c_char,
    control_name_len: size_t,
) {
    let target = target!(target, function_name!());
    let control_name = input(control_name, control_name_len).unwrap();
    target.clear_flow_control(control_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ clear_prefixed_flow_controls>](
    target: *mut $rust_target,
    control_prefix: *const c_char,
    control_prefix_len: size_t,
) {
    let target = target!(target, function_name!());
    let control_prefix = input(control_prefix, control_prefix_len).unwrap();
    target.clear_prefixed_flow_controls(control_prefix);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ set_flow_control_to_number_cstr>](
    target: *mut $rust_target,
    control_name: *const c_char,
    new_value: f32
) {
    let target = target!(target, function_name!());
    let control_name = input_cstr(control_name).unwrap();
    target.set_flow_control(control_name, StringOrNumber::Number(new_value));
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ set_flow_control_to_string_cstr>](
    target: *mut $rust_target,
    control_name: *const c_char,
    new_value: *const c_char,
) {
    let target = target!(target, function_name!());
    let control_name = input_cstr(control_name).unwrap();
    let new_value = input_cstr(new_value).unwrap();
    target.set_flow_control(control_name, StringOrNumber::String(new_value));
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ clear_flow_control_cstr>](
    target: *mut $rust_target,
    control_name: *const c_char,
) {
    let target = target!(target, function_name!());
    let control_name = input_cstr(control_name).unwrap();
    target.clear_flow_control(control_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ clear_prefixed_flow_controls_cstr>](
    target: *mut $rust_target,
    control_prefix: *const c_char,
) {
    let target = target!(target, function_name!());
    let control_prefix = input_cstr(control_prefix).unwrap();
    target.clear_prefixed_flow_controls(control_prefix);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ clear_all_flow_controls>](
    target: *mut $rust_target,
) {
    let target = target!(target, function_name!());
    target.clear_all_flow_controls();
}

// mix controls

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_mix_control_to>](
    target: *mut $rust_target,
    control_name: *const c_char,
    control_name_len: size_t,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let control_name = input(control_name, control_name_len).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_mix_control_to(control_name, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_mix_control_to_cstr>](
    target: *mut $rust_target,
    control_name: *const c_char,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let control_name = input_cstr(control_name).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_mix_control_to(control_name, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_prefixed_mix_controls_to>](
    target: *mut $rust_target,
    control_prefix: *const c_char,
    control_prefix_len: size_t,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let control_prefix = input(control_prefix, control_prefix_len).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_prefixed_mix_controls_to(control_prefix, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_prefixed_mix_controls_to_cstr>](
    target: *mut $rust_target,
    control_prefix: *const c_char,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let control_prefix = input_cstr(control_prefix).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_prefixed_mix_controls_to(control_prefix, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_all_mix_controls_to>](
    target: *mut $rust_target,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_all_mix_controls_to(positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_all_mix_controls_except_main_to>](
    target: *mut $rust_target,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_all_mix_controls_except_main_to(positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_mix_control_out>](
    target: *mut $rust_target,
    control_name: *const c_char,
    control_name_len: size_t,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let control_name = input(control_name, control_name_len).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_mix_control_out(control_name, positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_mix_control_out_cstr>](
    target: *mut $rust_target,
    control_name: *const c_char,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let control_name = input_cstr(control_name).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_mix_control_out(control_name, positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_prefixed_mix_controls_out>](
    target: *mut $rust_target,
    control_prefix: *const c_char,
    control_prefix_len: size_t,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let control_prefix = input(control_prefix, control_prefix_len).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_prefixed_mix_controls_out(control_prefix, positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_prefixed_mix_controls_out_cstr>](
    target: *mut $rust_target,
    control_prefix: *const c_char,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let control_prefix = input_cstr(control_prefix).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_prefixed_mix_controls_out(control_prefix, positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_all_mix_controls_out>](
    target: *mut $rust_target,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_all_mix_controls_out(positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_all_mix_controls_except_main_out>](
    target: *mut $rust_target,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_all_mix_controls_except_main_out(positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_mix_control>](
    target: *mut $rust_target,
    control_name: *const c_char,
    control_name_len: size_t,
) {
    let target = target!(target, function_name!());
    let control_name = input(control_name, control_name_len).unwrap();
    target.kill_mix_control(control_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_mix_control_cstr>](
    target: *mut $rust_target,
    control_name: *const c_char,
) {
    let target = target!(target, function_name!());
    let control_name = input_cstr(control_name).unwrap();
    target.kill_mix_control(control_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_prefixed_mix_controls>](
    target: *mut $rust_target,
    control_prefix: *const c_char,
    control_prefix_len: size_t,
) {
    let target = target!(target, function_name!());
    let control_prefix = input(control_prefix, control_prefix_len).unwrap();
    target.kill_mix_control(control_prefix);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_prefixed_mix_controls_cstr>](
    target: *mut $rust_target,
    control_prefix: *const c_char,
) {
    let target = target!(target, function_name!());
    let control_prefix = input_cstr(control_prefix).unwrap();
    target.kill_mix_control(control_prefix);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_all_mix_controls>](
    target: *mut $rust_target,
) {
    let target = target!(target, function_name!());
    target.kill_all_mix_controls();
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_all_mix_controls_except_main>](
    target: *mut $rust_target,
) {
    let target = target!(target, function_name!());
    target.kill_all_mix_controls_except_main();
}

// flows

#[no_mangle] #[named]
extern "C" fn [<$c_target _ start_flow>](
    target: *mut $rust_target,
    flow_name: *const c_char,
    flow_name_len: size_t,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_name = input(flow_name, flow_name_len).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.start_flow(flow_name, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ start_flow_cstr>](
    target: *mut $rust_target,
    flow_name: *const c_char,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_name = input_cstr(flow_name).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.start_flow(flow_name, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_flow_to>](
    target: *mut $rust_target,
    flow_name: *const c_char,
    flow_name_len: size_t,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_name = input(flow_name, flow_name_len).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_flow_to(flow_name, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_flow_to_cstr>](
    target: *mut $rust_target,
    flow_name: *const c_char,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_name = input_cstr(flow_name).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_flow_to(flow_name, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_prefixed_flows_to>](
    target: *mut $rust_target,
    flow_prefix: *const c_char,
    flow_prefix_len: size_t,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_prefix = input(flow_prefix, flow_prefix_len).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_prefixed_flows_to(flow_prefix, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_prefixed_flows_to_cstr>](
    target: *mut $rust_target,
    flow_prefix: *const c_char,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_prefix = input_cstr(flow_prefix).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_prefixed_flows_to(flow_prefix, positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_all_flows_to>](
    target: *mut $rust_target,
    target_volume: f32,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_all_flows_to(positive(target_volume), positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_flow_out>](
    target: *mut $rust_target,
    flow_name: *const c_char,
    flow_name_len: size_t,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_name = input(flow_name, flow_name_len).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_flow_out(flow_name, positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_flow_out_cstr>](
    target: *mut $rust_target,
    flow_name: *const c_char,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_name = input_cstr(flow_name).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_flow_out(flow_name, positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_prefixed_flows_out>](
    target: *mut $rust_target,
    flow_prefix: *const c_char,
    flow_prefix_len: size_t,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_prefix = input(flow_prefix, flow_prefix_len).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_prefixed_flows_out(flow_prefix, positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_prefixed_flows_out_cstr>](
    target: *mut $rust_target,
    flow_prefix: *const c_char,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let flow_prefix = input_cstr(flow_prefix).unwrap();
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_prefixed_flows_out(flow_prefix, positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ fade_all_flows_out>](
    target: *mut $rust_target,
    fade_length: f32,
    fade_type: c_int,
) {
    let target = target!(target, function_name!());
    let fade_type = fade_type!(fade_type, function_name!());
    target.fade_all_flows_out(positive(fade_length), fade_type);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_flow>](
    target: *mut $rust_target,
    flow_name: *const c_char,
    flow_name_len: size_t,
) {
    let target = target!(target, function_name!());
    let flow_name = input(flow_name, flow_name_len).unwrap();
    target.kill_flow(flow_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_flow_cstr>](
    target: *mut $rust_target,
    flow_name: *const c_char,
) {
    let target = target!(target, function_name!());
    let flow_name = input_cstr(flow_name).unwrap();
    target.kill_flow(flow_name);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_prefixed_flows>](
    target: *mut $rust_target,
    flow_prefix: *const c_char,
    flow_prefix_len: size_t,
) {
    let target = target!(target, function_name!());
    let flow_prefix = input(flow_prefix, flow_prefix_len).unwrap();
    target.kill_flow(flow_prefix);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_prefixed_flows_cstr>](
    target: *mut $rust_target,
    flow_prefix: *const c_char,
) {
    let target = target!(target, function_name!());
    let flow_prefix = input_cstr(flow_prefix).unwrap();
    target.kill_flow(flow_prefix);
}

#[no_mangle] #[named]
extern "C" fn [<$c_target _ kill_all_flows>](
    target: *mut $rust_target,
) {
    let target = target!(target, function_name!());
    target.kill_all_flows();
}

}}}

implement_commands!(SMS_Engine, Engine);
implement_commands!(SMS_Commander, Commander);
implement_commands!(SMS_Transaction, Transaction<'static, dyn EngineCommandIssuer>);