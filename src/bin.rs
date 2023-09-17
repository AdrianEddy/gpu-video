// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

use gpu_video::*;
use std::collections::HashMap;
use std::io::Write;

fn main() {
    let _time = std::time::Instant::now();

    let _ = simple_log::new(simple_log::LogConfig::default());

    let mut decoder = Decoder::new("E:/__GH011230.MP4", DecoderOptions {
        gpu_index: Some(4),
        ranges_ms: Vec::new(),
        custom_options: HashMap::new()
    }).unwrap();

    for stream in decoder.streams() {
        println!("stream {stream:?}");
        if stream.index != 0 {
            stream.decode = false;
        }
    }

    while let Some(mut frame) = decoder.next_frame() {
        match &mut frame {
            Frame::Video(v) => {
                println!("Video frame at {:?}: {}x{}: {:?}", v.timestamp_us(), v.width(), v.height(), v.format());
                // for buf in v.get_cpu_buffers().unwrap() {
                //     println!("buf len: {}", buf.len());
                // }
            },
            Frame::Audio(v) => {
                println!("Audio frame at {:?}", v.timestamp_us());
            },
            _ => {
                // println!("Other frame");
            }
        }
    }

    println!("Done in {:.3}s ", _time.elapsed().as_millis() as f64 / 1000.0);
    std::io::stdout().flush().unwrap();
}
