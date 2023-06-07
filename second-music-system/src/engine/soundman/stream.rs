use super::*;

/// Manages cached *streams*. We initialize decoders for them, prerolled to
/// the requested start positions, and dish them out on request. We use a lot
/// more CPU time, but a lot less memory.
pub struct StreamManager {
}

impl SoundMan for StreamManager {
    
}