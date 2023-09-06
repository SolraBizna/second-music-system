use super::*;

mod fadeadapter;
use fadeadapter::*;
mod chanadapter;
use chanadapter::*;
mod rateadapter;
use rateadapter::*;

pub(crate) fn adaptify(
    delegate: &Arc<dyn SoundDelegate>,
    soundman: &mut SoundMan,
    sound: &Sound,
    fade_in: f32, length: Option<f32>, fade_out: f32,
    out_sample_rate: f32, out_speaker_layout: SpeakerLayout,
) -> Option<Box<dyn SoundReader<f32>>> {
    eprintln!("Get {:?}?", sound.name);
    let stream = soundman.get_sound(sound)?;
    eprintln!("Got!");
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
    let mut stream = new_fade_adapter(sound, stream, fade_in, length, fade_out);
    let need_chan_adapter = in_speaker_layout != out_speaker_layout;
    let num_channels = if need_chan_adapter && in_sample_rate < out_sample_rate {
        stream = new_channel_adapter(stream, in_sample_rate, in_speaker_layout, out_speaker_layout);
        out_speaker_layout.get_num_channels()
    } else { in_speaker_layout.get_num_channels() };
    if in_sample_rate != out_sample_rate {
        stream = new_rate_adapter(delegate, stream, num_channels as u32, in_sample_rate, out_sample_rate);
    }
    if need_chan_adapter && in_sample_rate >= out_sample_rate {
        stream = new_channel_adapter(stream, out_sample_rate, in_speaker_layout, out_speaker_layout);
    }
    Some(stream)
}
