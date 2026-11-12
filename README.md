# rustic_audio_lean

Voice notes for Lean 4 programs: record a clip from the default microphone to an Ogg Opus file
(48 kHz, mono, 20 ms frames), play one back, and play or hear raw samples. It wraps the Rust
crate [Rustic Audio](https://github.com/katzenpost/rustic_audio) over
[lean-sys](https://github.com/katzenpost/lean-sys). It is for Lean programmers who need sound
in an executable; a Rust program should use Rustic Audio directly.

It uses the default input and output only, records or plays one clip at a time, and runs on
Linux through ALSA. It does not choose devices or write formats other than Ogg Opus.

## Use

Install `cargo`, ALSA's headers and `pkg-config` (`apt install libasound2-dev pkg-config`).
In your `lakefile.toml`:

```toml
[[require]]
name = "rustic_audio_lean"
git = "https://github.com/katzenpost/rustic_audio_lean"
rev = "<commit>"

[[lean_exe]]
name = "app"
moreLinkArgs = ["-Wl,--allow-shlib-undefined", "/usr/lib/x86_64-linux-gnu/libasound.so.2"]
```

and in `app.lean`:

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

Then `lake build app && .lake/build/bin/app`. Raw samples are in
[docs/raw-samples.md](docs/raw-samples.md); why ALSA is linked by path, sound devices and
releases are in [docs/building.md](docs/building.md).

## Map

- [`RusticAudio.lean`](RusticAudio.lean): the Lean API, `Engine` and `Clip`.
- [`RusticAudio/Tests.lean`](RusticAudio/Tests.lean): the `audio-test` executable.
- [`rust/src/lib.rs`](rust/src/lib.rs): the exported functions, over Rustic Audio.
- [`rust/src/stream.rs`](rust/src/stream.rs): raw samples and the resampler.
- [`tests/tone.opus`](tests/tone.opus): a clip the tests play.

## Status

Version 0.2.0, on Lean 4.35.0 and Rust 1.99.0. `abiMatches` checks that the linked Rust
library is the one the Lean declarations are for (ABI version 2). CI also builds the crate for
Android (arm64).

## Tests

    (cd rust && cargo test)
    lake exe audio-test

Without a sound device the tests that play and record are skipped; ALSA's null device runs them
without hardware, as described in [docs/building.md](docs/building.md).

## License

[AGPL-3.0-only](LICENSE)
