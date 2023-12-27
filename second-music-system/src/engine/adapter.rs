use super::*;

mod fadeadapter;
use fadeadapter::*;
mod chanadapter;
use chanadapter::*;
#[cfg(feature = "resample-soxr")]
#[path = "adapter/rateadapter_soxr.rs"]
mod rateadapter;
#[cfg(not(feature = "resample-soxr"))]
#[path = "adapter/rateadapter_terrible.rs"]
mod rateadapter;
use rateadapter::*;

#[allow(clippy::too_many_arguments)] // (internal function, doesn't care)
pub(crate) fn adaptify(
    delegate: &Arc<dyn SoundDelegate>,
    soundman: &mut dyn GenericSoundMan,
    sound: &Sound,
    fade_in: PosFloat,
    length: Option<PosFloat>,
    fade_out: PosFloat,
    out_sample_rate: PosFloat,
    out_speaker_layout: SpeakerLayout,
) -> Option<Box<dyn SoundReader<f32>>> {
    let stream = soundman.get_sound(sound)?;
    let in_sample_rate = stream.sample_rate;
    let in_speaker_layout = stream.speaker_layout;
    /*
    // TODO: if it's already an F32 stream and has no interesting fade, use it
    // directly. Something like:
    let mut stream = match &stream.reader {
        FormattedSoundReader::F32(x) if length.is_none() && fade_in == 0.0 && fade_out == 0.0 => stream.reader,
        _ => new_fade_adapter(sound, stream, fade_in, length, fade_out, release),
    };
    // We can do this safely with sounds that come from BufferMan, but not
    // necessarily with sounds that come from StreamMan!
    */
    let mut stream = new_fade_adapter(
        sound,
        stream,
        fade_in,
        length.or_else(|| {
            sound.end.get().map(|x| x.saturating_sub(sound.start))
        }),
        fade_out,
    );
    let need_chan_adapter = in_speaker_layout != out_speaker_layout;
    let num_channels = if need_chan_adapter && in_sample_rate < out_sample_rate
    {
        stream = new_channel_adapter(
            stream,
            in_sample_rate,
            in_speaker_layout,
            out_speaker_layout,
        );
        out_speaker_layout.get_num_channels()
    } else {
        in_speaker_layout.get_num_channels()
    };
    if in_sample_rate != out_sample_rate {
        stream = new_rate_adapter(
            delegate,
            stream,
            num_channels as u32,
            in_sample_rate,
            out_sample_rate,
        );
    }
    if need_chan_adapter && in_sample_rate >= out_sample_rate {
        stream = new_channel_adapter(
            stream,
            out_sample_rate,
            in_speaker_layout,
            out_speaker_layout,
        );
    }
    Some(stream)
}
