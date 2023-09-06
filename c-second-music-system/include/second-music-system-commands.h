#include <stddef.h>

// Every command in this file can either be sent to an `SMS_Engine`, an
// `SMS_Commander`, or an `SMS_Transaction`. They are named
// accordingly. For example, all of the following send the `set_flow_control`
// command:
//
// - `SMS_Engine_set_flow_control`
// - `SMS_Commander_set_flow_control`
// - `SMS_Transaction_set_flow_control`

#define SMS_Cat(a,b) a##_##b
#define SMS_IndirectCat(a,b) SMS_Cat(a,b)
#define SMS_Command(name) SMS_IndirectCat(SMS_Target, name)

// Starts a new transaction. Commands that are issued will be batched
// together into one transaction, and delivered and processed all at once
// when the transaction is complete.
// 
// - `length`: Your best guess as to the number of commands that will be
//   sent during this transaction. This is an optimization hint only. Specify
//   zero to refuse to guess.
//
// You must either abort or commit the transaction.
struct SMS_Transaction* SMS_Command(begin_transaction)(
    struct SMS_Target*,
    size_t length
);

// Replace the active soundtrack with the given one. Currently-active
// nodes, sequences, and sounds will do their best to play to their
// conclusion.
//
// If you're replacing one soundtrack with an entirely different one, you
// probably want to fade or stop all flows first. If you're replacing it
// with a variation of the current soundtrack, such as one that contains
// additional flows, this replacement is seamless.
//
// Note: Passing an `SMS_Soundtrack*` to this function transfers ownership of
// that object to SMS. You can no longer use that same object again. If you
// want to retain a copy for yourself, `clone` your soundtrack and submit the
// clone instead. Do *not* call `SMS_Soundtrack_free` on this pointer!
void SMS_Command(replace_soundtrack)(
    struct SMS_Target*,
    struct SMS_Soundtrack* new_soundtrack
);

// Requests that the given flow be precached for playback. The engine
// will attempt to load/preroll all requested sounds and streams in the
// background. Use TODO to determine when the loading is complete.
// 
// This is *not* recursive. If you call `precache` twice, then call
// `unprecache` once, the flow will no longer be precached.
void SMS_Command(precache)(
    struct SMS_Target*,
    const char* flow_name,
    size_t flow_name_len
);
void SMS_Command(precache_cstr)(
    struct SMS_Target*,
    const char* flow_name
);

// Undoes a previous request that the given flow be precached for
// playback. This will lead the relevant sounds and streams to be purged
// once the flow stops playing (or immediately, if the flow is
// not currently playing).
// 
// Commands sent from a given thread are always received in order, so it
// is completely reasonable to call `start_flow` immediately followed
// by `unprecache` for the same flow.
// 
// This is *not* recursive. If you call `precache` twice, then call
// `unprecache` once, the flow will no longer be precached.
void SMS_Command(unprecache)(
    struct SMS_Target*,
    const char* flow_name,
    size_t flow_name_len
);
void SMS_Command(unprecache_cstr)(
    struct SMS_Target*,
    const char* flow_name
);

// Undoes all previous requests for precaching of flows. Flows that are
// currently in use will still remain in memory.
//
// Commands sent from a given thread are always received in order, so it
// is completely reasonable to call `start_flow` immediately followed
// by `unprecache_all`.
void SMS_Command(unprecache_all)(
    struct SMS_Target*
);

// Sets a given FlowControl to the given value.
void SMS_Command(set_flow_control_to_number)(
    struct SMS_Target*,
    const char* control_name,
    size_t control_name_len,
    float new_value
);
void SMS_Command(set_flow_control_to_number_cstr)(
    struct SMS_Target*,
    const char* control_name,
    float new_value
);
void SMS_Command(set_flow_control_to_string)(
    struct SMS_Target*,
    const char* control_name,
    size_t control_name_len,
    const char* new_value,
    size_t new_value_len
);
void SMS_Command(set_flow_control_to_string_cstr)(
    struct SMS_Target*,
    const char* control_name,
    const char* new_value
);

// Clears a given FlowControl, removing any previous value.
void SMS_Command(clear_flow_control)(
    struct SMS_Target*,
    const char* control_name,
    size_t control_name_len
);
void SMS_Command(clear_flow_control_cstr)(
    struct SMS_Target*,
    const char* control_name
);

// Clears all FlowControls whose names strictly start with the given
// prefix.
void SMS_Command(clear_prefixed_flow_controls)(
    struct SMS_Target*,
    const char* control_prefix,
    size_t control_prefix_len
);
void SMS_Command(clear_prefixed_flow_controls_cstr)(
    struct SMS_Target*,
    const char* control_prefix
);

// Clears all FlowControls.
void SMS_Command(clear_all_flow_controls)(
    struct SMS_Target*
);

// Fades a given MixControl to the given volume (0.0 to 1.0), using the
// given fading curve, over the given time period (in seconds).
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals. Don't give a volume above 1.0 unless you are sure
// it won't cause clipping. Don't give negative volumes.
void SMS_Command(fade_mix_control_to)(
    struct SMS_Target*,
    const char* control_name,
    size_t control_name_len,
    float target_volume,
    float fade_length,
    int fade_type
);
void SMS_Command(fade_mix_control_to_cstr)(
    struct SMS_Target*,
    const char* control_name,
    float target_volume,
    float fade_length,
    int fade_type
);

// Fades all *currently existing* mix controls whose names strictly
// start with the given prefix to the given volume (0.0 to 1.0), using the
// given fading curve, over the given time period (in seconds).
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals. Don't give a volume above 1.0 unless you are sure
// it won't cause clipping. Don't give negative volumes.
void SMS_Command(fade_prefixed_mix_controls_to)(
    struct SMS_Target*,
    const char* control_prefix,
    size_t control_prefix_len,
    float target_volume,
    float fade_length,
    int fade_type
);
void SMS_Command(fade_prefixed_mix_controls_to_cstr)(
    struct SMS_Target*,
    const char* control_prefix,
    float target_volume,
    float fade_length,
    int fade_type
);

// Fades all *currently existing* mix controls, *including* `main`, to
// the given volume (0.0 to 1.0), using the given fading curve, over the
// given time period (in seconds).
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals. Don't give a volume above 1.0 unless you are sure
// it won't cause clipping. Don't give negative volumes.
void SMS_Command(fade_all_mix_controls_to)(
    struct SMS_Target*,
    float target_volume,
    float fade_length,
    int fade_type
);

// Fades all *currently existing* mix controls, *except* `main`, to the
// given volume (0.0 to 1.0), using the given fading curve, over the given
// time period (in seconds).
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals. Don't give a volume above 1.0 unless you are sure
// it won't cause clipping. Don't give negative volumes.
void SMS_Command(fade_all_mix_controls_except_main_to)(
    struct SMS_Target*,
    float target_volume,
    float fade_length,
    int fade_type
);

// Fades a given MixControl to zero volume, using the given fading curve,
// over the given time period (in seconds). When the fade is complete, the
// MixControl will be removed from existence rather than simply zeroed;
// future commands to "prefixed" and "all" will not resuscitate it (unless
// it is the target of a future, specific command).
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals.
void SMS_Command(fade_mix_control_out)(
    struct SMS_Target*,
    const char* control_name,
    size_t control_name_len,
    float fade_length,
    int fade_type
);
void SMS_Command(fade_mix_control_out_cstr)(
    struct SMS_Target*,
    const char* control_name,
    float fade_length,
    int fade_type
);

// Fades all *currently existing* mix controls whose names strictly
// start with the given prefix to zero volume, using the given fading
// curve, over the given time period (in seconds). When the fade is
// complete, the MixControl will be removed from existence rather than
// simply zeroed; future commands to "prefixed" and "all" will not
// resuscitate it (unless it is the target of a future, specific command).
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals.
void SMS_Command(fade_prefixed_mix_controls_out)(
    struct SMS_Target*,
    const char* control_prefix,
    size_t control_prefix_len,
    float fade_length,
    int fade_type
);
void SMS_Command(fade_prefixed_mix_controls_out_cstr)(
    struct SMS_Target*,
    const char* control_prefix,
    float fade_length,
    int fade_type
);

// Fades all *currently existing* mix controls, *including* `main`,
// to zero volume, using the given fading curve, over the given time
// period (in seconds). When the fade is complete, the MixControl will be
// removed from existence rather than simply zeroed; future commands to
// "prefixed" and "all" will not resuscitate it (unless it is the target
// of a future, specific command).
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals.
void SMS_Command(fade_all_mix_controls_out)(
    struct SMS_Target*,
    float fade_length,
    int fade_type
);

// Fades all *currently existing* mix controls, *except* `main`,
// to zero volume, using the given fading curve, over the given time
// period (in seconds). When the fade is complete, the MixControl will be
// removed from existence rather than simply zeroed; future commands to
// "prefixed" and "all" will not resuscitate it (unless it is the target
// of a future, specific command).
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals.
void SMS_Command(fade_all_mix_controls_except_main_out)(
    struct SMS_Target*,
    float fade_length,
    int fade_type
);

// Kills a given MixControl instantly, as if you yanked an audio cable.
// 
// This is similar to fading that MixControl out over zero seconds, except
// that the MixControl in question is immediately removed (and therefore
// ineligible for `prefixed` or `all` commands), instead of only being
// removed the next time mixing takes place.
void SMS_Command(kill_mix_control)(
    struct SMS_Target*,
    const char* control_name,
    size_t control_name_len
);
void SMS_Command(kill_mix_control_cstr)(
    struct SMS_Target*,
    const char* control_name
);

// Kills all MixControls whose names strictly start with the given prefix,
// as if you yanked an audio cable.
// 
// This is similar to fading that MixControl out over zero seconds, except
// that the MixControl in question is immediately removed (and therefore
// ineligible for `prefixed` or `all` commands), instead of only being
// removed the next time mixing takes place.
void SMS_Command(kill_prefixed_mix_controls)(
    struct SMS_Target*,
    const char* control_prefix,
    size_t control_name_len
);
void SMS_Command(kill_prefixed_mix_controls_cstr)(
    struct SMS_Target*,
    const char* control_prefix
);

// Kills all MixControls, *including* `main`, as if you yanked an audio
// cable.
// 
// This is similar to fading that MixControl out over zero seconds, except
// that the MixControl in question is immediately removed (and therefore
// ineligible for `prefixed` or `all` commands), instead of only being
// removed the next time mixing takes place.
void SMS_Command(kill_all_mix_controls)(
    struct SMS_Target*
);

// Kills all MixControls, *except* `main`, as if you yanked an audio
// cable.
// 
// This is similar to fading that MixControl out over zero seconds, except
// that the MixControl in question is immediately removed (and therefore
// ineligible for `prefixed` or `all` commands), instead of only being
// removed the next time mixing takes place.
void SMS_Command(kill_all_mix_controls_except_main)(
    struct SMS_Target*
);

// Starts a given flow if it's not already playing. If the flow
// is being newly started, it will be faded up from zero volume to the
// target volume, with the given fade curve. If the flow was
// already playing, acts just like `fade_flow_to`.
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals. Don't give a volume above 1.0 unless you are sure
// it won't cause clipping. Don't give negative volumes.
void SMS_Command(start_flow)(
    struct SMS_Target*,
    const char* flow_name,
    size_t flow_name_len,
    float target_volume,
    float fade_length,
    int fade_type
);
void SMS_Command(start_flow_cstr)(
    struct SMS_Target*,
    const char* flow_name,
    float target_volume,
    float fade_length,
    int fade_type
);

// Fades a given flow to the given volume (0.0 to 1.0), using the
// given fading curve, over the given time period (in seconds). Does
// nothing if the flow is not currently playing.
// 
// Flows with zero volume will continue silently "playing", waiting to
// be faded back up to non-zero volume. If this isn't what you want, use
// `fade_flow_out` instead.
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals. Don't give a volume above 1.0 unless you are sure
// it won't cause clipping. Don't give negative volumes.
void SMS_Command(fade_flow_to)(
    struct SMS_Target*,
    const char* flow_name,
    size_t flow_name_len,
    float target_volume,
    float fade_length,
    int fade_type
);
void SMS_Command(fade_flow_to_cstr)(
    struct SMS_Target*,
    const char* flow_name,
    float target_volume,
    float fade_length,
    int fade_type
);

// Fades all *currently playing* flows whose names strictly start with
// the given prefix to the given volume (0.0 to 1.0), using the given
// fading curve, over the given time period (in seconds). Does nothing to
// flows that haven't been started, or that have finished fading out.
// 
// Flows with zero volume will continue silently "playing", waiting to
// be faded back up to non-zero volume. If this isn't what you want, use
// `fade_prefixed_flows_out` instead.
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals. Don't give a volume above 1.0 unless you are sure
// it won't cause clipping. Don't give negative volumes.
void SMS_Command(fade_prefixed_flows_to)(
    struct SMS_Target*,
    const char* flow_prefix,
    size_t flow_prefix_len,
    float target_volume,
    float fade_length,
    int fade_type
);
void SMS_Command(fade_prefixed_flows_to_cstr)(
    struct SMS_Target*,
    const char* flow_prefix,
    float target_volume,
    float fade_length,
    int fade_type
);

// Fades all *currently playing* flows to the given volume (0.0 to
// 1.0), using the given fading curve, over the given time period (in
// seconds). Does nothing to flows that haven't been started, or that
// have finished fading out.
// 
// Flows with zero volume will continue silently "playing", waiting to
// be faded back up to non-zero volume. If this isn't what you want, use
// `fade_prefixed_flows_out` instead.
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals. Don't give a volume above 1.0 unless you are sure
// it won't cause clipping. Don't give negative volumes.
void SMS_Command(fade_all_flows_to)(
    struct SMS_Target*,
    float target_volume,
    float fade_length,
    int fade_type
);

// Fades a given flow to zero volume, using the given fading curve,
// over the given time period (in seconds). Does nothing if the flow
// is not currently playing, or has already faded out. When the fade is
// complete, the flow will be stopped.
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals.
void SMS_Command(fade_flow_out)(
    struct SMS_Target*,
    const char* flow_name,
    size_t flow_name_len,
    float fade_length,
    int fade_type
);
void SMS_Command(fade_flow_out_cstr)(
    struct SMS_Target*,
    const char* flow_name,
    float fade_length,
    int fade_type
);

// Fades all *currently playing* flows whose names strictly start with
// the given prefix to zero volume, using the given fading curve, over the
// given time period (in seconds). Does nothing to flows that haven't
// been started, or that have already finished fading out.
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals.
void SMS_Command(fade_prefixed_flows_out)(
    struct SMS_Target*,
    const char* flow_prefix,
    size_t flow_prefix_len,
    float fade_length,
    int fade_type
);
void SMS_Command(fade_prefixed_flows_out_cstr)(
    struct SMS_Target*,
    const char* flow_prefix,
    float fade_length,
    int fade_type
);

// Fades all *currently playing* flows to zero volume, using the given
// fading curve, over the given time period (in seconds). Does nothing to
// flows that haven't been started, or that have already finished
// fading out.
// 
// Use `SMS_FADE_TYPE_EXPONENTIAL` unless you are doing intermixing of
// correlated signals.
void SMS_Command(fade_all_flows_out)(
    struct SMS_Target*,
    float fade_length,
    int fade_type
);

// Kills a given flow instantly.
// 
// This is similar to fading that flow out over zero seconds, except
// that the flow in question is immediately removed (and therefore
// ineligible for `prefixed` or `all` commands, and able to be started
// from the beginning), instead of only being removed the next time mixing
// takes place.
void SMS_Command(kill_flow)(
    struct SMS_Target*,
    const char* flow_name,
    size_t flow_name_len
);
void SMS_Command(kill_flow_cstr)(
    struct SMS_Target*,
    const char* flow_name
);

// Kills all *currently playing* flows whose names strictly start with
// the given prefix instantly.
// 
// This is similar to fading that flow out over zero seconds, except
// that the flow in question is immediately removed (and therefore
// ineligible for `prefixed` or `all` commands, and able to be started
// from the beginning), instead of only being removed the next time mixing
// takes place.
void SMS_Command(kill_prefixed_flows)(
    struct SMS_Target*,
    const char* flow_prefix,
    size_t flow_prefix_len
);
void SMS_Command(kill_prefixed_flows_cstr)(
    struct SMS_Target*,
    const char* flow_prefix
);

// Kills all *currently playing* flows instantly.
// 
// This is similar to fading those flows out over zero seconds, except
// that the flows in question are immediately removed (and therefore
// ineligible for `prefixed` or `all` commands, and able to be started
// from the beginning), instead of only being removed the next time mixing
// takes place.
void SMS_Command(kill_all_flows)(
    struct SMS_Target*
);

#undef SMS_Command
#undef SMS_IndirectCat
#undef SMS_Cat
