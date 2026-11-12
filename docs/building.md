# Building, linking and releases

## Building

Building needs `cargo` on the `PATH`, and ALSA's headers and `pkg-config`
(`apt install libasound2-dev pkg-config`). Lake builds the crate in `rust/` with
`cargo build --release` and links it into your executables. The Rust toolchain is pinned in
`rust/rust-toolchain.toml`.

## Linking ALSA

Executables link ALSA's shared library by its path:

```toml
moreLinkArgs = ["-Wl,--allow-shlib-undefined", "/usr/lib/x86_64-linux-gnu/libasound.so.2"]
```

Lean links with its own, older C library, so the symbols ALSA takes from the system's C library
are left to be found when the program starts.

## Sound devices

Sound goes through ALSA's default devices; PulseAudio and PipeWire are reached through their
ALSA plugins. Reading `/dev/snd` directly needs membership of the `audio` group.

ALSA's null device plays and records silence, which runs the tests without hardware:

    printf 'pcm.!default { type null }\nctl.!default { type hw card 0 }\n' > null.conf
    ALSA_CONFIG_PATH=null.conf lake exe audio-test

## Releases

A release is a tag `vX.Y.Z` naming the `version` in `lakefile.lean`; pushing the tag runs the
tests and makes the GitHub release. Reservoir lists the package's versions from these tags.
