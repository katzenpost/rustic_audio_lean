/-
SPDX-FileCopyrightText: Copyright (C) 2026 David Stainton
SPDX-License-Identifier: AGPL-3.0-only
-/

import RusticAudio

/-! # Tests

The engine through the FFI, without sound hardware; and, where there is an output device,
playing a clip. -/

open RusticAudio

def check (name : String) (ok : Bool) : StateT Nat IO Unit := do
  if ok then IO.println s!"ok   {name}"
  else IO.println s!"FAIL {name}"; modify (· + 1)

def fails (act : IO α) : IO Bool := do
  try discard act; return false catch _ => return true

def main : IO UInt32 := do
  let dir := (← IO.currentDir) / ".lake" / "audio-test"
  -- Clips an earlier run left would be counted with this run's.
  if ← dir.pathExists then IO.FS.removeDirAll dir
  let ((), failed) ← (do
    check "the library is the one declared" abiMatches
    let e ← Engine.new dir.toString
    check "the engine makes its folder" (← dir.isDir)
    check "nothing is recording" !(← e.isRecording)
    check "nothing is playing" !(← e.isPlaying)
    check "stopping a recording never started fails" (← fails e.stopCapture)
    check "cancelling one is nothing" !(← e.cancelCapture)
    check "playing a missing file fails" (← fails (e.play (dir / "missing.opus").toString))
    check "no playback error yet" ((← e.takePlaybackError).isNone)
    discard <| e.stopPlayback
    let made ← fails (Engine.new "/proc/no-such/dir")
    check "an engine where no folder can be made fails" made
    let tone := (← IO.currentDir) / "tests" / "tone.opus"
    match ← (e.play tone.toString).toBaseIO with
    | .error err => IO.println s!"skip playing a clip: {err}"
    | .ok () =>
      IO.sleep 300
      let err ← e.takePlaybackError
      if let some m := err then IO.println s!"skip playing a clip: {m}"
      else
        e.stopPlayback
        check "and stops" !(← e.isPlaying)
    match ← (e.startCapture (some "test")).toBaseIO with
    | .error err => IO.println s!"skip recording a clip: {err}"
    | .ok path =>
      check "a recording is named after its stem" ((path.splitOn "/").getLast!.startsWith "test-")
      check "and is recording" (← e.isRecording)
      check "a second recording at once fails" (← fails (e.startCapture none))
      IO.sleep 1000
      match ← e.stopCapture.toBaseIO with
      | .error err => IO.println s!"skip recording a clip: {err}"
      | .ok clip =>
        check "a clip with sound in it" (clip.durationMs > 0)
        let bytes ← IO.FS.readBinFile clip.path
        check "its file is Ogg Opus of its size" (bytes.size == clip.size && bytes.extract 0 4 == "OggS".toUTF8)
        check "nothing is recording after" !(← e.isRecording)
        discard <| e.startCapture none
        check "a recording cancelled is thrown away" (← e.cancelCapture)
        check "and leaves only the clip kept" ((← dir.readDir).size == 1)).run 0
  IO.println (if failed == 0 then "all passed" else s!"{failed} failed")
  return if failed == 0 then 0 else 1
