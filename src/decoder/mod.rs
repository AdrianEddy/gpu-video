// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

pub(crate) mod ffmpeg; use ffmpeg::*;
pub(crate) mod braw;   use braw::*;

use crate::*;
use crate::types::VideoProcessingError;

use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct DecoderOptions {
    pub gpu_index: Option<usize>,
    pub ranges_ms: Vec<(f32, f32)>,
    pub custom_options: HashMap<String, String>,
}

#[enum_delegate::register]
pub trait DecoderInterface {
    fn streams(&mut self) -> Vec<&mut Stream>;
    fn seek(&mut self, timestamp_us: i64) -> Result<bool, VideoProcessingError>;

    fn next_frame(&mut self) -> Result<Option<Frame>, VideoProcessingError>;

    fn get_video_info(&self) -> Result<VideoInfo, VideoProcessingError>;
}

pub struct Decoder {
    inner: DecoderBackend
}

impl Decoder {
    pub fn new(path: &str, options: DecoderOptions) -> Result<Self, VideoProcessingError> {
        if path.to_ascii_lowercase().ends_with(".braw") {
            return Ok(Self {
                inner: DecoderBackend::BrawDecoder(BrawDecoder::new(path, options)?)
            });
        }
        Ok(Self {
            inner: DecoderBackend::FfmpegDecoder(FfmpegDecoder::new(path, options)?)
        })
    }

    pub fn streams(&mut self) -> Vec<&mut Stream> {
        self.inner.streams()
    }
    pub fn seek(&mut self, timestamp_us: i64) -> Result<bool, VideoProcessingError> {
        self.inner.seek(timestamp_us)
    }
    pub fn next_frame(&mut self) -> Result<Option<Frame>, VideoProcessingError> {
        self.inner.next_frame()
    }
    pub fn get_video_info(&self) -> Result<VideoInfo, VideoProcessingError> {
        self.inner.get_video_info()
    }
}

#[enum_delegate::implement(DecoderInterface)]
pub enum DecoderBackend {
    FfmpegDecoder(FfmpegDecoder),
    BrawDecoder(BrawDecoder)
}
