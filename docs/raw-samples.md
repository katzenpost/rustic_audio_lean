# Raw samples

For codecs such as ggwave, an engine also plays and hears raw samples: mono, in -1..1, at a
rate of your choosing, resampled to and from whatever rate the device runs at.

```lean
  e.playSamples 48000 samples          -- a FloatArray; returns once it has played
  e.startListen 48000 1024             -- the microphone, in frames of 1024 samples
  if let some frame ← e.readFrame then -- none until a whole frame is ready
    IO.println s!"{frame.size} samples"
  e.stopListen
```

Up to two seconds wait to be read; a reader that falls further behind loses what comes after.
The resampler is linear interpolation, in `rust/src/stream.rs`.
