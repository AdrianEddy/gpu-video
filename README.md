# gpu-video
Rust library for decoding and encoding video on the GPU. Designed to be backend-independent, but ffmpeg will be the main focus at the beginning

WARNING: NOT READY YET. This will be a refactor of [Gyroflow](https://github.com/gyroflow/gyroflow/)'s ffmpeg code. Currently only ffmpeg decoder works

Designed to have very simple yet powerful API.

Example:
```rust
let mut decoder = Decoder::new("video_file.mp4", DecoderOptions {
    gpu_index: Some(0),
    ranges_ms: Vec::new(),
    custom_options: HashMap::new()
}).unwrap();

for stream in decoder.streams() {
    println!("Stream {stream:?}");
    if stream.index != 0 { // Decode only first stream
        stream.decode = false;
    }
}

while let Some(mut frame) = decoder.next_frame() {
    match &mut frame {
        Frame::Video(v) => {
            println!("Video frame at {:?}: {}x{}: {:?}", v.timestamp_us(), v.width(), v.height(), v.format());
            // Download from GPU to CPU buffer by simply calling `get_cpu_buffers`
            for buf in v.get_cpu_buffers().unwrap() {
                println!("Buffer size: {}", buf.len());
            }
        },
        Frame::Audio(v) => {
            println!("Audio frame at {:?}", v.timestamp_us());
        },
        _ => {
            // println!("Other frame");
        }
    }
}
```

# Get started
1. Install Just: `cargo install just`
2. Download and extract ffmpeg: `just install-deps`
3. Compile and run: `just run`

# Features

- Decoders
    - [x] ffmpeg
    - [ ] BRAW
    - [ ] RED RAW
    - [ ] VideoToolbox
    - [ ] MFT
    - [ ] MediaCodec

- Conversion
    - [ ] ffmpeg
    - [ ] zimg
    - [ ] wgpu

- Encoders
    - [ ] ffmpeg
    - [ ] VideoToolbox
    - [ ] MFT
    - [ ] MediaCodec

----------------

Future plans, separate crates:
- Processing
    - [ ] cpu
    - [ ] wgpu
    - [ ] CUDA
- Player:
    - play
    - pause
    - stop
    - seek
    - setPlaybackRate
- Audio:
    - ffmpeg -> cpal
    - miniaudio


#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>