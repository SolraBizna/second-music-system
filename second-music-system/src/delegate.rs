use super::FormattedSoundStream;

/// This is an object that SMS will hang onto, and will call upon to open sound
/// files and issue warnings. It must be thread safe.
pub trait SoundDelegate: Send + Sync {
    /// Attempt to open an sound file with the given name. If it doesn't exist,
    /// an IO error occurs, you can't identify the format, or whatever, you
    /// should display or log an error message using an application-specific
    /// mechanism, then return `None`.
    fn open_file(&self, name: &str) -> Option<FormattedSoundStream>;
    /// Present and/or log a warning in some application-specific way.
    fn warning(&self, message: &str) {
        eprintln!("SMS warning: {}", message);
    }
}
