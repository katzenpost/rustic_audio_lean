import Lake
open Lake DSL System

package «rustic_audio_lean» where
  version := v!"0.1.0"
  description := "Voice notes for Lean 4, from rustic_audio"
  keywords := #["audio", "voice", "ffi", "rust"]
  license := "AGPL-3.0-only"

@[default_target]
lean_lib RusticAudio

lean_exe «audio-test» where
  root := `RusticAudio.Tests
  moreLinkArgs := #["-Wl,--allow-shlib-undefined", "/usr/lib/x86_64-linux-gnu/libasound.so.2"]

/-- The Rust crate in `rust/`, built by cargo. -/
extern_lib rustic_audio_lean pkg := Job.async do
  proc { cmd := "cargo", args := #["build", "--release", "--quiet"], cwd := pkg.dir / "rust" }
  let lib := pkg.dir / "rust" / "target" / "release" / nameToStaticLib "rustic_audio_lean"
  setTrace (← computeTrace lib)
  return lib
