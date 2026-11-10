/-
SPDX-FileCopyrightText: Copyright (C) 2026 David Stainton
SPDX-License-Identifier: AGPL-3.0-only
-/

/-! # Voice notes

Recording a clip from the default microphone to an Ogg Opus file (48 kHz, mono, 20 ms frames),
and playing such a file on the default output, from the Rust crate Rustic Audio. The functions
are implemented in `rust/src/lib.rs`. -/

namespace RusticAudio

def abiVersion : UInt32 := 2

@[extern "rustic_audio_abi_version"] opaque rustAbiVersion : Unit → UInt32

/-- Whether the linked Rust library is the one these declarations are for. -/
def abiMatches : Bool := rustAbiVersion () == abiVersion

opaque Engine.Pointed : NonemptyType

/-- Records one clip and plays one clip at a time. Recording and playing happen on threads of
its own; dropping the last reference stops both. -/
def Engine : Type := Engine.Pointed.type

instance : Nonempty Engine := Engine.Pointed.property

/-- A recorded clip. -/
structure Clip where
  path : String
  durationMs : Nat
  size : Nat
  deriving Repr, BEq, Inhabited

/-- An engine that keeps its recordings in `dir`, which it creates. -/
@[extern "ra_engine_new"] opaque Engine.new (dir : @& String) : IO Engine

/-- Starts recording, stopping any playback; the path the clip will have, in the engine's
folder, named after `stem`. -/
@[extern "ra_start_capture"]
opaque Engine.startCapture (e : @& Engine) (stem : @& Option String) : IO String

/-- Stops recording and encodes the clip. Blocks for the encoding, half a second or more. -/
@[extern "ra_stop_capture"] opaque Engine.stopCapture (e : @& Engine) : IO Clip

/-- Stops recording and deletes the clip; whether one was being recorded. -/
@[extern "ra_cancel_capture"] opaque Engine.cancelCapture (e : @& Engine) : IO Bool

/-- Starts playing an Ogg Opus file of 48 kHz mono, stopping whatever was playing. -/
@[extern "ra_play"] opaque Engine.play (e : @& Engine) (path : @& String) : IO Unit

@[extern "ra_stop_playback"] opaque Engine.stopPlayback (e : @& Engine) : IO Unit

@[extern "ra_is_recording"] opaque Engine.isRecording (e : @& Engine) : IO Bool

/-- Whether a clip is playing; false again once it has played to its end. -/
@[extern "ra_is_playing"] opaque Engine.isPlaying (e : @& Engine) : IO Bool

/-- Why the last playback failed, once. -/
@[extern "ra_take_playback_error"]
opaque Engine.takePlaybackError (e : @& Engine) : IO (Option String)

/-! ## Raw samples -/

/-- Plays mono samples in -1..1 at `rate` Hz on the default output, resampled to the device's
rate. Blocks until they have played. -/
@[extern "ra_play_samples"]
opaque Engine.playSamples (e : @& Engine) (rate : UInt32) (samples : @& FloatArray) : IO Unit

/-- Starts listening to the default microphone, heard mono at `rate` Hz in frames of `frame`
samples. Up to two seconds wait to be read; what comes after is dropped until they are. -/
@[extern "ra_start_listen"]
opaque Engine.startListen (e : @& Engine) (rate frame : UInt32) : IO Unit

/-- The next frame heard, if one is ready. Fails when not listening. -/
@[extern "ra_read_frame"] opaque Engine.readFrame (e : @& Engine) : IO (Option FloatArray)

/-- Stops listening, dropping what was heard and not read. -/
@[extern "ra_stop_listen"] opaque Engine.stopListen (e : @& Engine) : IO Unit

@[extern "ra_is_listening"] opaque Engine.isListening (e : @& Engine) : IO Bool

end RusticAudio
