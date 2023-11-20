// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

mod ffmpeg; use ffmpeg::*;

use crate::*;
use crate::types::VideoProcessingError;

use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct DecoderOptions {
    pub gpu_index: Option<usize>,
    pub ranges_ms: Vec<(f32, f32)>,
    pub custom_options: HashMap<String, String>,
}

#[derive(Debug, Copy, Clone)]
pub enum StreamType {
    Video,
    Audio,
    Subtitle,
    Other
}

#[derive(Debug, Clone)]
pub struct Stream {
    pub stream_type: StreamType,
    pub index: usize,
    pub time_base: (i32, i32),
    pub avg_frame_rate: (i32, i32),
    pub rate: (i32, i32),

    pub decode: bool,
}

#[enum_delegate::register]
pub trait DecoderInterface {
    fn streams(&mut self) -> Vec<&mut Stream>;
    fn seek(&mut self, timestamp_us: i64) -> bool;

    fn next_frame(&mut self) -> Option<Frame>;

    fn get_video_info(&self) -> Result<VideoInfo, VideoProcessingError>;
}

pub struct Decoder {
    inner: DecoderBackend
}

impl Decoder {
    pub fn new(path: &str, options: DecoderOptions) -> Result<Self, VideoProcessingError> {
        Ok(Self {
            inner: DecoderBackend::FfmpegDecoder(FfmpegDecoder::new(path, options)?)
        })
    }

    pub fn streams(&mut self) -> Vec<&mut Stream> {
        self.inner.streams()
    }
    pub fn next_frame(&mut self) -> Option<Frame> {
        self.inner.next_frame()
    }
    pub fn get_video_info(&mut self) -> Result<VideoInfo, VideoProcessingError> {
        self.inner.get_video_info()
    }
}

#[enum_delegate::implement(DecoderInterface)]
pub enum DecoderBackend {
    FfmpegDecoder(FfmpegDecoder)
}
