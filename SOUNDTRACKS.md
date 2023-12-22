The beating heart of SMS is the Engine, and its lifeblood is the Soundtrack. A Soundtrack contains all of the information about a particular musical soundscape apart from the actual audio data.

# Structure

A Soundtrack contains four kinds of data:

- Sounds
- Sequences
- Nodes
- Flows

## Sound

A **Sound** is a particular audio file, or a particular passage of a particular audio file. It's the fundamental building block of all other parts of the soundtrack, since it's the only way to get actual sound to come out of your speakers.

Example sound (more details below):

```ini
sound "Drum Loop"
  file "drums.opus"
```

When triggered, it simply plays from beginning to end. The same Sound can be playing more than once at the same time (usually by accident).

## Sequence

A **Sequence** is a set of instructions to play one or more Sounds or other Sequences at particular times, in particular channels. There's no logic at this level. [NEMO PUT A METAPHOR HERE PLEASE]

Example sequence, to be combined with the example sound above (more details below):

```ini
timebase beats 128/m

sequence "Drums With Trumpets"
  length 8 beats
  play sound "Drum Loop"
    at 0 beats
  play sound "Drum Loop"
    at 4 beats
  play sound
    file "trumpet1.opus"
    at 2 beats
```

Once a Sequence is triggered, it will proceed from beginning to end, triggering all of the other Sounds and Sequences that it is programmed to trigger. The same Sequence can be playing more than once at the same time (usually by accident).

## Flow

A **Flow** describes which Sounds and Sequences to play, at what times, under what conditions. It will usually correspond to one song, though you can use it in other ways as well, such as by having multiple songs in a single Flow or by building up one song out of multiple Flows.

Example Flows are given below.

Unlike Sounds and Sequences, only one instance of a particular Flow can be playing at the same time. You can't make the same Flow overlap itself by playing it twice.

## Nodes

A **Node** is a branch in the Flow. If you make a flowchart of a particular dynamic song, the flowchart as a whole is the Flow and each line in the flowchart is a Node inside that flow.

Example Nodes are given below.

Each Flow has at least one Node, the "starting node", which is where playback begins when that Flow is started. It may *optionally* have more Nodes, which can run in parallel and can trigger each other according to logic. It may also, optionally, automatically restart at the beginning of the starting node whenever there are no Nodes still playing.

# Syntax by example

Soundtracks are described using the Second Music System Soundtrack Language. Numerous example Soundtracks are given below.

The basic syntax has lines with words, separated by spaces, and related to one another by indentation. You can use single or double quotes, or backslash escapes, to combine multiple words into one.

Here is the simplest useful Soundtrack. It contains one Flow, which simply plays the same audio track on loop until it is stopped:

```ini
# Everything after "#" is a comment, and is ignored by SMS.
flow "Simple Example" with loop
  play sound and wait # you usually want "and wait"
    file "simple_example.opus"
```

The flow is named "Simple Example", and that is the name by which the script will request it. It makes no use of flow or mix controls, so all the script can do to influence it from there is to change its volume or stop it completely.

Here's a more complex example:

```ini
flow "More Complex Example" with loop
  play sound and wait
    file "track1.opus"
  play sound and wait
    file "track2.opus"
  play sound and wait
    file "track3.opus"
```

This will play the first track, then the second track, then the third track, and then loop back to the first track again. Still no controls, and the script can still only start playback, stop playback, or change its volume.

Let's look at a more interesting example:

```ini
flow "Dungeon BGM"
  play sound and wait
    file "dungeon_intro.opus"
  start node "Main" # THIS causes the node "Main" to be started at this point.
  node "Main"       # ...and THIS defines the node "Main".
    if $underwater then switch to node "Underwater"
    play sound and wait
      file "dungeon_main1.opus"
    if $underwater then switch to node "Underwater"
    play sound and wait
      file "dungeon_main2.opus"
    restart node "Main"
  node "Underwater"
    if !$underwater then switch to node "Main"
    play sound and wait
      file "dungeon_underwater.mp3"
    restart node "Underwater"
```

This will first play "dungeon_intro.opus", and then alternate between playing "dungeon_main1.opus" and "dungeon_main2.opus" if "underwater" is false, and "dungeon_underwater.mp3" if "underwater" is true. It will still loop forever, but since we didn't specify "with loop", it only loops because "Main" and "Underwater" explicitly restart themselves when they're finished.

To make this work, the game's script can set the "underwater" flow control to a non-zero value if the player is underwater and to zero if the player isn't. But the playback will only change at the end of a soundfile. That's not very responsive.

To improve the response time, use a mix control instead of a flow control. This requires us to introduce Sequences, which in turn requres us to talk about timebases:

```ini
# Define the timebase "beats" as having 128 per minute. Since this is the only
# timebase in the soundtrack, this will be used whenever we don't specify a
# timebase.
timebase beats 128/m

flow "Dungeon Boss"
  play sound and wait
    file "dungeon_boss_intro.opus"
  start node "Battle"
  node "Battle"
    if $"boss died" then switch node "Dying"
    play sequence and wait
      length 4 # four beats, one measure
      play sound
        file "dungeon_boss_1_main.opus"
        # we don't specify a channel, so it defaults to "main"
      play sound
        file "dungeon_boss_1_hazard.opus"
        channel "hazard"
      play sound
        file "dungeon_boss_1_vulnerable.opus"
        channel "vulnerable"
    if $"boss died" then switch node "Dying"
    play sequence and wait
      length 4
      play sound
        file "dungeon_boss_2_main.opus"
      play sound
        file "dungeon_boss_2_hazard.opus"
        channel "hazard"
      play sound
        file "dungeon_boss_2_vulnerable.opus"
        channel "vulnerable"
    restart node "Battle"
  node "Dying"
    play sound and wait "dungeon_boss_dying.opus"
    # since we don't start any other nodes, playback will end once this track
    # is over
```

When this flow is started, we will play an introduction, and then play a three-layered track that responds to how the battle is going.

The script will control which layers are active and in what proportions by setting the "main", "hazard", and "vulnerable" **mix** controls to values between 0 (silent) and 1 (full volume). **Mix** controls act like sliders on a mixing board.

Once the boss is defeated, the game will set the "boss died" **flow** control to a non-zero value, which means that at the next measure boundary we will switch to the "boss dying" track and then the song will end.


