#ifndef SECOND_MUSIC_SYSTEM_H
#define SECOND_MUSIC_SYSTEM_H

#include <stdlib.h>
#include <stdint.h>

#if __cplusplus
extern "C" {
#endif

// Version of SMS that this header file is for.
#define SMS_HEADER_VERSION_STRING "0.1.0"
// 0xMMmmPP; M = major, m = minor, P = patch
#define SMS_HEADER_VERSION_NUMBER ((uint32_t)((0x00 << 16) | (0x01 << 8) | 0x00))
// Version of SMS that you are linked to.
const char* SMS_get_version_string();
uint32_t SMS_get_version_number();

// One channel, one speaker.
#define SMS_SPEAKER_LAYOUT_MONO 0
// Two channels, two speakers. (FL, FR)
#define SMS_SPEAKER_LAYOUT_STEREO 1
// Two channels, headphones. (L, R)
#define SMS_SPEAKER_LAYOUT_HEADPHONES 2
// Four channels, speakers in each corner. (FL, FR, RL, RR)
#define SMS_SPEAKER_LAYOUT_QUADRAPHONIC 3
// Six channels. Speakers in each corner, one in front, and one subwoofer.
// (FL, FR, C, LFE, RL, RR)
#define SMS_SPEAKER_LAYOUT_SURROUND51 4
// Eight channels. Speakers in each corner, one in front, one on each
// side, and one subwoofer. (FL, FR, C, LFE, RL, RR, SL, SR)
#define SMS_SPEAKER_LAYOUT_SURROUND71 5

// Unsigned 8-bit sound. Zero point is 128, extremes are 1 and 255
#define SMS_SOUND_FORMAT_UNSIGNED_8 0
// Unsigned 16-bit sound. Zero point is 32768, extremes are 1 and 65535
#define SMS_SOUND_FORMAT_UNSIGNED_16 1
// Signed 8-bit sound. Zero point is 0, extremes are -127 and +127
#define SMS_SOUND_FORMAT_SIGNED_8 2
// Signed 16-bit sound. Zero point is 0, extremes are -32767 and +32767
#define SMS_SOUND_FORMAT_SIGNED_16 3
// IEEE 754 32-bit float sound. Zero point is 0, extremes are -1 and +1
#define SMS_SOUND_FORMAT_FLOAT_32 4

// Fades between the given volumes on a logarithmic curve, such that any
// given timespan within the fade will have the same perceived volume
// change as any other.
#define SMS_FADE_TYPE_LOGARITHMIC 1
// Fades linearly between the given amplification factors. You only want
// this when you're crossfading between partly correlated samples.
#define SMS_FADE_TYPE_LINEAR 2
// Fades between the given volumes on an exponential curve, resulting in
// a fade that "hangs out" at the louder side. Arguably more
// aesthetically pleasing than a logarithmic fade.
#define SMS_FADE_TYPE_EXPONENTIAL 0

#define SMS_FADE_TYPE_DEFAULT SMS_FADE_TYPE_EXPONENTIAL

// Strings:
//
// Any function that takes strings comes in two variants. In the regular
// variant, strings can contain null bytes, but you must explicitly pass in the
// length. In the `_cstr` variant, strings are C-style null-terminated strings.
// Use whichever one you want.

// Error handling:

// Functions that return pointers will return NULL on error. Functions that
// return int will return 0 on error.
//
// Fallible functions will have an `error_out` and `error_len_out` parameter.
//
// `error_out` parameter, if non-NULL, is filled in with a newly malloc'd C
// string containing the error text (including a null terminator). You must
// free this when you're done with it.
//
// `error_len_out` parameter, if non-NULL, is filled in with the length of the
// new error (not including the null terminator).
//
// If no error occurs, they are *not touched*.

///////////////////////////////////////////////////////////////////////////////
// Soundtrack
///////////////////////////////////////////////////////////////////////////////
// Encapsulates all the information about a soundtrack: what files to play,
// how to play them, etc. This is purely inert data. It can be built up
// incrementally, or replaced entirely, cleanly and efficiently.
struct SMS_Soundtrack;

struct SMS_Soundtrack* SMS_Soundtrack_new();
void SMS_Soundtrack_free(struct SMS_Soundtrack*);

struct SMS_Soundtrack* SMS_Soundtrack_clone(struct SMS_Soundtrack*);

// If parsing fails, returns NULL.
struct SMS_Soundtrack* SMS_Soundtrack_parse_new(const char* src, size_t src_len, char** error_out, size_t* error_len_out);
struct SMS_Soundtrack* SMS_Soundtrack_parse_new_cstr(const char* src, char** error_out, size_t* error_len_out);
// Changes the soundtrack, adding new elements and replacing same-named ones.
// If parsing fails, leaves the existing soundtrack alone.
int SMS_Soundtrack_parse(struct SMS_Soundtrack*, const char* src, size_t src_len, char** error_out, size_t* error_len_out);
int SMS_Soundtrack_parse_cstr(struct SMS_Soundtrack*, const char* src, char** error_out, size_t* error_len_out);

///////////////////////////////////////////////////////////////////////////////
// FormattedSoundStream
///////////////////////////////////////////////////////////////////////////////
// Describes a sound stream actively being decoded from the game data. It has
// a particular sample rate (which we will convert), a particular speaker
// layout (which we may also convert), and a callback that will return decoded
// samples as needed. SMS will either cache this or stream it directly...
// because of the latter case, mind your thread safety!
struct SMS_FormattedSoundStream;

struct SMS_FormattedSoundStream* SMS_FormattedSoundStream_new(
    // Passed as the first parameter to all stream callbacks.
    void* callback_data,
    // Samples per second of the original audio data. (SMS will resample this
    // if needed.)
    float sample_rate,
    // Speaker layout of the original audio data. (SMS will up-/downmix this
    // if needed.)
    //
    // Must be one of the SMS_SPEAKER_LAYOUT_* constants above.
    int speaker_layout,
    // Format of the audio data we will be outputting.
    //
    // Must be one of the SMS_SOUND_FORMAT_* constants above.
    int format,
    // Produce some sound, placing it into the target buffer.
    // 
    // Return the number of *samples* (not *sample frames*) that were written
    // to buf. If this is not *exactly* equal to the size of the buf, then the
    // stream is assumed to have been ended; either it will be disposed of,
    // or `seek` will be called.
    //
    // Note: The given size_t value is the number of samples in the buffer,
    // *not* the number of sample *frames*, nor the number of *bytes*!
    //
    // Note 2: While all other handlers may be NULL, this one *must* be
    // non-NULL. (You almost certainly want `free_handler` to be non-NULL too.)
    size_t (*read_handler)(void*, void* buf, size_t num_samples_in_buf),
    // Called when this stream is no longer referenced anymore. May
    // be NULL.
    void (*free_handler)(void*),
    // Attempt to seek to the given *sample frame count* from the beginning of
    // the file. Imprecision is permitted in one direction only: seeking is
    // permitted to end up earlier than the target, but not later. Returns the
    // actual *sample frame count*, measured from the beginning of the stream,
    // that was seeked to.
    //
    // This number must be exact! If you can't provide an exact timestamp,
    // don't provide seeking! (SMS will work around it.) Again, it's okay if
    // you can't *seek to an exact timestamp*, but you *do* need to be able to
    // *know where you've seeked to* and *not seek too late*.
    //
    // Returns `(uint64_t)-1` if seeking failed or is impossible, in which
    // case, SMS will reopen the file instead. Default implementation returns
    // this.
    //
    // SMS may call `seek(0)` upon opening your stream, to determine if
    // seeking is something your decoder supports. You should return a value
    // only if you are confident that seeking will succeed in the future.
    //
    // **IMPORTANT NOTE**: Do not implement this if you can seek only forward.
    // Do not special case successful seek when coincidentally seeking to
    // where the cursor already is. *Do not* implement this by calling your
    // own skip routines. If you disregard this warning, SMS will **panic**
    // the first time it attempts to loop a stream. If you can only seek
    // forward, implement only the skip routines.
    //
    // You also shouldn't implement this function if seeking is as expensive
    // as reopening the file, starting decoding from scratch, and calling
    // `skip_*`. If this is the case, just don't implement it. SMS has logic
    // that can do that work in a background thread.
    uint64_t (*seek_handler)(void*, uint64_t position),
    // Attempt to skip exactly the given number of *samples*. Failure is not
    // an option. Returns non-zero if there is more sound data to come, zero if
    // we have reached the end of the sound.
    //
    // The default implementation will try to use `skip_coarse` to skip
    // ahead, and then repeatedly `read` into the target buffer until the
    // exact number of target samples are consumed.
    // 
    // `buf` is provided as scratch space.
    //
    // Note: The given size_t value is the number of samples in the buffer,
    // *not* the number of sample *frames*, nor the number of *bytes*!
    int (*skip_precise_handler)(void*, uint64_t count, void* buf, size_t buf_len),
    // Attempt to efficiently skip *up to* a large number of *samples*, by
    // discarding partial buffers, skipping packets, seeking in the file,
    // etc. Return the number of *samples* skipped, possibly including zero.
    // 
    // Default implementation just returns 0.
    // 
    // `buf` is provided as scratch space.
    //
    // Note: The given size_t value is the number of samples in the buffer,
    // *not* the number of sample *frames*, nor the number of *bytes*!
    uint64_t (*skip_coarse_handler)(void*, uint64_t count, void* buf, size_t buf_len),
    // If this handler is non-NULL, this is a cloneable decoder, and this
    // handler will be called if a clone is needed. If this handler is NULL,
    // this is a non-cloneable decoder.
    //
    // Sample rate and speaker layout are provided in case you do not keep
    // track of your own sample rate and speaker layout internally.
    //
    // Cloning cannot fail. If you say you can clone, you better mean it.
    struct SMS_FormattedSoundStream* (*clone_handler)(void*, float sample_rate, int speaker_layout),
    // Attempt to estimate how many *sample frames* are in the entire file,
    // from beginning to end. This is a BEST GUESS ESTIMATE and may not
    // reflect the actual value!
    //
    // SMS will never call this after it has seeked/skipped/read any audio
    // data, so it is safe for implementors to assume the cursor is at the
    // beginning of the file and give less accurate data otherwise.
    //
    // Return (uint64_t)-1 if the estimation process failed, or just specify a
    // NULL handler in the first place if you can't make estimates at all.
    uint64_t (*estimate_len_handler)(void*)
);
// SMS will handle freeing the sound stream when it's done with it. You only
// need this function if you have created a new `FormattedSoundStream`, but
// have changed your mind about passing it along to SMS.
void SMS_FormattedSoundStream_free(struct SMS_FormattedSoundStream*);

///////////////////////////////////////////////////////////////////////////////
// SoundDelegate
///////////////////////////////////////////////////////////////////////////////
// This is an object that SMS will hang onto, and will call upon to open sound
// files and issue warnings. It must be thread safe.
struct SMS_SoundDelegate;

struct SMS_SoundDelegate* SMS_SoundDelegate_new(
    // Passed as the first parameter to all delegate callbacks.
    void* callback_data,
    // Attempt to open an sound file with the given name. If it doesn't exist,
    // an IO error occurs, you can't identify the format, or whatever, you
    // should display or log an error message using an application-specific
    // mechanism, then return NULL.
    struct SMS_FormattedSoundStream*(*file_open_handler)(void*, const char* name),
    // Present and/or log a warning in some application-specific way.
    // May be NULL, in which case warnings will be printed directly to stderr.
    void(*warning_handler)(void*, const char* message),
    // Called when this SMS_SoundDelegate is no longer referenced anymore. May
    // be NULL.
    void(*free_handler)(void*)
);
// Call this when you are done with *your copy* of the pointer to this
// SoundDelegate, i.e. when you know you are not going to pass it to any more
// new Engines.
//
// SMS_SoundDelegate is actually a reference counted pointer. The delegate
// will not actually be freed until any and all Engines that were created with
// it have also been freed.
void SMS_SoundDelegate_free(struct SMS_SoundDelegate*);

///////////////////////////////////////////////////////////////////////////////
// Engine
///////////////////////////////////////////////////////////////////////////////
// This is the main moving part of the Second Music System. You create one of
// these, give it a delegate to handle music decoding, and "turn the handle"
// in your sound output code to make music come out.
struct SMS_Engine;

// Creates a new Engine with no soundtrack and no controls. Once these
// properties are set, they cannot be changed without creating a new
// Engine.
// 
// - `speaker_layout`: What kind of speaker layout your listener has. When
//   in doubt, use `SMS_SPEAKER_LAYOUT_STEREO`.
// - `sample_rate`: Number of samples per second you will be outputting.
// - `num_threads`: Number of threads to use for decoding and streaming.
//   If 0, will use a reasonable default based on the number of available
//   hardware threads and whether background loading is requested.
// - `background_loading`: Should be non-zero if you're in a realtime
//   context, like a game, zero if you're in a batch context, like
//   recording a pre-rendered video.
struct SMS_Engine* SMS_Engine_new(
    struct SMS_SoundDelegate* sound_delegate,
    int speaker_layout,
    float sample_rate,
    int num_threads,
    int background_loading
);
void SMS_Engine_free(struct SMS_Engine*);

// Makes an independent `Commander` that can send commands to this
// `Engine` from another thread.
struct SMS_Commander* SMS_Engine_clone_commander(
    struct SMS_Engine*
);

// Get a copy of the Soundtrack that is currently live
// (You will have to free it when you're done)
struct SMS_Soundtrack* SMS_Engine_copy_live_soundtrack(
    struct SMS_Engine*
);

// TODO: Engine::copy_all_flow_controls

// Returns the `SpeakerLayout` this `Engine` was initialized for.
int SMS_Engine_get_speaker_layout(struct SMS_Engine*);
// Returns the sample rate this `Engine` was initialized for.
float SMS_Engine_get_sample_rate(struct SMS_Engine*);
// Returns whether this `Engine` was initialized with background loading
// turned on or off.
int SMS_Engine_is_loading_in_background(struct SMS_Engine*);

// Mix some audio, advance time! `out` must have a number of elements
// divisible by the number of speaker channels. Any existing data in `out`
// is mixed with the active music data. You may or may not want to zero
// `out` before this call.
//
// `out_len` is the number of ELEMENTS, i.e. SAMPLES, in the output buffer.
// It is NOT the number of bytes, and it is NOT the number of frames.
void SMS_Engine_turn_handle(
    struct SMS_Engine*,
    float* out,
    size_t out_len
);

#define SMS_Target SMS_Engine
#include "second-music-system-commands.h"
#undef SMS_Target

///////////////////////////////////////////////////////////////////////////////
// Commander
///////////////////////////////////////////////////////////////////////////////
// This exists to send commands to an `Engine` that belongs to some other
// thread. If you're operating entirely in a single thread, you can also just
// call the equivalent methods on an `Engine` directly.
struct SMS_Commander;

void SMS_Commander_free(struct SMS_Commander*);

// Makes another, independent `Commander` that sends commands to the
// same underlying `Engine`.
struct SMS_Commander* SMS_Engine_clone_commander(
    struct SMS_Engine*
);

#define SMS_Target SMS_Commander
#include "second-music-system-commands.h"
#undef SMS_Target

///////////////////////////////////////////////////////////////////////////////
// Transaction
///////////////////////////////////////////////////////////////////////////////
// An in-progress transaction. Create one by calling any of the
// `SMS_*_begin_transaction` functions. (See `second-music-system-commands.h`)
//
// You can send commands to a transaction, exactly like you can to an
// `Engine`. When you `commit` a transaction, all of the commands will be sent
// at once, atomically, with neither a gap nor any interleaving with any other
// commands. You can instead `abort` a transaction, in which case none of the
// commands will be sent.
struct SMS_Transaction;

// Free a transaction, without executing any of its commands.
void SMS_Transaction_abort(struct SMS_Commander*);
// Send the accumulated commands off to be executed, and then free the
// transaction.
void SMS_Transaction_commit(struct SMS_Commander*);

#define SMS_Target SMS_Transaction
#include "second-music-system-commands.h"
#undef SMS_Target

///////////////////////////////////////////////////////////////////////////////
// Utilities
///////////////////////////////////////////////////////////////////////////////

// Pass in one of the SMS_SPEAKER_LAYOUT_* constants above. This will return
// how many channels it has, or 0 if an invalid value is given.
int SMS_SpeakerLayout_get_num_channels(int);

#if __cplusplus
}
#endif

#endif
