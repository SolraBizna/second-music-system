Second Music System is a middleware library focused on dynamic music. It gives a lot of flexibility to composers, while being as easy as possible to integrate into an existing game engine, partly by having no opinions on the details of audio input or output.

It is still in an early phase of development. Documentation and tooling are sparse. Performance is good, but not yet what it could be. The API and soundtrack format may be subject to change.

# For composers

With Second Music System, you can make complex compositions that shift and branch in response to conditions and events in the game, with minimal support from programmers or scripters. Unfortunately, this currently requires you to work in SMS's [soundtrack language](SOUNDTRACKS.md) directly, and thus requires a little bit of "programming". Tooling may make this a little easier in the future, but the fundamental heart of an SMS soundtrack is always going to be this language.

# For scripters

With Second Music System, you can expose as much or as little information as you like about your game status to the soundtrack. Composers can use as much or as little of that information as they want, and all of this power is available with very little work from the engine itself.

# For programmers

With Second Music System, all you have to do is:

- Provide a hook for loading music files, in whatever data format and structure is natural for your engine.
- Periodically accept samples from the SMS `Engine` and send them to the output ("turn the handle").
- Send commands to the `Engine` to load soundtracks, start/stop flows, set flow/mix controls, etc. either by writing code that does those things directly, or by exposing those calls to your engine's scripting environment.

You *don't* have to write code to handle fades, crossfades, branches, logic for multithreaded loading, or any of that. SMS will handle the grunt work. You just need to provide some plumbing and SMS will do the rest.

# FAQ

## Features?

You can crossfade between related sounds (like _Faster Than Light_), branch between movements (like _Starfleet Academy_ or _Halo_), overlap multiple independent flows (like a certain infamous bug in _Halo_'s last level), and weave together sounds of varying lengths and repetitions.

You **can't**: apply complicated filters or realtime synthesis, including but not limited to speed and tempo changes. (Some filters, such as equalization, can be accomplished by doing some filtering ahead of time and then having SMS mix differently filtered versions of the same sound in different ratios.)

You **currently can't** but will someday be able to: pause a flow and then resume where it left off.

## Rust?

Second Music System is written in Rust, and that's the best language to use to interface with it. However, nearly\* all of its features are also available in the form of a C binding. If your game engine is written in another language, you can use the C binding as a basis.

A C++ binding based on the C binding is planned but isn't written yet.

\*(some complicated state queries are not bound yet, but we hope to fix that soon)

## Queries?

SMS is designed under the assumption that audio processing is happening "in the background", separately from the rest of your game logic. It's architected to make it so that these sides never wait for each other. The audio thread must never block waiting for another thread to do something, and vice versa. Unfortunately this makes "reading" the state of the SMS engine outside the audio thread more complicated than one might expect.

For the rare situation where you regularly, and constantly, need to read the same information about the SMS soundtrack's current state, there is a simple approach. Once per tick:

- If you've not sent this query yet, send one.
- If you have sent this query at least once, and a response has arrived, cache that response and send a fresh query.
- Use the cached response for this tick.

## Recordings?

SMS is designed to work equally well for realtime playback as well as offline recording. Its performance is optimized for the realtime case, but should be acceptable for offline use as well. For consistent recording results, you should ask SMS to perform "foreground loading", which means it will always load things synchronously as needed. For realtime use this would result in small hitches, but for recording, this means that tracks will consistently be available, and there will be no delays that might vary from recording to recording.

- If using the Rust API, use `new_with_runtime` with the `ForegroundTaskRuntime` when doing a recording.
- If using the C API (which doesn't expose `Runtime`s), pass zero for the `background_loading` parameter for `SMS_Engine_new` when doing a recording.

## Delays!

SMS loves to load things in the background, and it does not love making anything wait for anything. When you start playback of a flow, it will start loading/precharging all necessary components for that flow, and playback won't actually begin until they're ready. This is fine for level background music and the like, but is a major problem for stings, and other things that need to be ready to go at a moment's notice.

You can avoid delays by using `precache` to keep those flows ready to go. Simply `precache` all the music you're going to need "soon" and it will be ready and waiting when you need it.

If your game has loading screens, and you want to sit at the loading screen until SMS is finished loading a certain set of flows, `precache` all those flows and then periodically query SMS to see if those flows are loaded.

## Memory?

SMS is not very memory hungry, except for the teensy problem that sounds are preloaded and predecoded in their entirety by default. This reduces loading-related hitches and CPU usage, at the cost of *needing to store the decompressed version of every soundtrack component in memory*. Individual sounds can request that they be streamed instead. They will still be "precharged" in the background, but most moment-to-moment IO and decoding will now be happening on the audio thread. For very complex flows, or those containing very long sounds, this may be worth it for the memory savings.

## CPU?

Second Music System performs all loading in background threads. It will automatically use several hardware threads to do loading in parallel if available. **Streamed sounds are currently decoded in only one thread**, but a future version will use multithreading here as well, with no change to the API.

## FMOD.

Yes, FMOD is more widely known and supported and, likely, both easier to use and more performant than SMS. It also has several nice features that SMS doesn't have and never will have. But FMOD is closed source commercial software that uses a proprietary, secret format; Second Music System is—

# Legalese

Second Music System is copyright 2022 and 2023 Solra Bizna and Noah Obert. It is licensed under either of:

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or
   <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the Second Music System crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
