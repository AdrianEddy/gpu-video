// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

#[cfg(feature = "ffmpeg")] pub(crate) mod ffmpeg;
#[cfg(feature = "braw")] pub(crate) mod braw;
#[cfg(feature = "r3d")]  pub(crate) mod r3d;

use crate::*;
use crate::types::VideoProcessingError;

use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct DecoderOptions {
    pub gpu_index: Option<usize>,
    pub custom_options: HashMap<String, String>,
}

#[enum_dispatch::enum_dispatch(DecoderBackend)]
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
        #[cfg(feature = "braw")]
        if path.to_ascii_lowercase().ends_with(".braw") {
            return Ok(Self {
                inner: DecoderBackend::BrawDecoder(braw::BrawDecoder::new(path, options)?)
            });
        }
        #[cfg(feature = "r3d")]
        if path.to_ascii_lowercase().ends_with(".r3d") || path.to_ascii_lowercase().ends_with(".nev") {
            return Ok(Self {
                inner: DecoderBackend::R3dDecoder(r3d::R3dDecoder::new(path, options)?)
            });
        }
        #[cfg(feature = "ffmpeg")]
        {
            return Ok(Self {
                inner: DecoderBackend::FfmpegDecoder(ffmpeg::FfmpegDecoder::new(path, options)?)
            });
        }

        Err(VideoProcessingError::DecoderNotFound)
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

#[enum_dispatch::enum_dispatch]
pub enum DecoderBackend {
    Unknown(NullDecoder),
    #[cfg(feature = "ffmpeg")]
    FfmpegDecoder(ffmpeg::FfmpegDecoder),
    #[cfg(feature = "braw")]
    BrawDecoder(braw::BrawDecoder),
    #[cfg(feature = "r3d")]
    R3dDecoder(r3d::R3dDecoder)
}

pub struct NullDecoder;

impl DecoderInterface for NullDecoder {
    fn streams(&mut self) -> Vec<&mut Stream> { Vec::new() }
    fn seek(&mut self, timestamp_us: i64) -> Result<bool, VideoProcessingError> { Ok(false) }
    fn next_frame(&mut self) -> Result<Option<Frame>, VideoProcessingError> { Ok(None) }
    fn get_video_info(&self) -> Result<VideoInfo, VideoProcessingError> { Err(VideoProcessingError::DecoderNotFound) }
}
