# rustic_audio_lean

Voice notes for Lean 4 programs: recording a clip from the default microphone to an Ogg Opus
file (48 kHz, mono, 20 ms frames) and playing one, from the Rust crate
[Rustic Audio](https://github.com/katzenpost/rustic_audio). The Rust side uses
[lean-sys](https://github.com/katzenpost/lean-sys) for Lean's object model.

## Use

In your `lakefile`:

```toml
[[require]]
name = "rustic_audio_lean"
git = "https://github.com/katzenpost/rustic_audio_lean"
rev = "<commit>"

[[lean_exe]]
name = "app"
moreLinkArgs = ["-Wl,--allow-shlib-undefined", "/usr/lib/x86_64-linux-gnu/libasound.so.2"]
```

```lean
import RusticAudio
open RusticAudio

def main : IO Unit := do
  let e ← Engine.new "notes"
  discard <| e.startCapture (some "hello")
  IO.sleep 3000
  let clip ← e.stopCapture
  IO.println s!"{clip.path}: {clip.durationMs} ms, {clip.size} bytes"
  e.play clip.path
  while ← e.isPlaying do IO.sleep 100
```

Building needs `cargo` on the `PATH`, and ALSA's headers and `pkg-config`
(`apt install libasound2-dev pkg-config`). Lake builds the crate in `rust/` and links it into
your executables.

Executables link ALSA's shared library by its path, as above. Lean links with its own, older C
library, so the symbols ALSA takes from the system's C library are left to be found when the
program starts.

Sound goes through ALSA's default devices; PulseAudio and PipeWire are reached through their
ALSA plugins. Reading `/dev/snd` directly needs membership of the `audio` group.

## Tests

    lake exe audio-test

Without a sound device the tests that play and record are skipped. ALSA's null device runs
them without hardware:

    printf 'pcm.!default { type null }\nctl.!default { type hw card 0 }\n' > null.conf
    ALSA_CONFIG_PATH=null.conf lake exe audio-test

## Releases

A release is a tag `vX.Y.Z` naming the `version` in `lakefile.lean`; pushing the tag runs the
tests and makes the GitHub release. Reservoir lists the package's versions from these tags.

## License

AGPL-3.0-only
