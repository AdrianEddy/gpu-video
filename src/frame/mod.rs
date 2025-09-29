// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

mod ffmpeg; pub use ffmpeg::*;
mod braw; pub use braw::*;
use crate::types::*;

pub struct TextureDescription {
    pub texture: HWTexture,
}

#[enum_delegate::register]
pub trait VideoFrameInterface {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn timestamp_us(&self) -> Option<i64>;
    fn format(&self) -> PixelFormat;
    fn get_cpu_buffers(&mut self) -> Result<Vec<&mut [u8]>, crate::VideoProcessingError>;
    fn get_gpu_texture(&mut self, plane: usize) -> Option<TextureDescription>;
}

#[enum_delegate::implement(VideoFrameInterface)]
pub enum VideoFrame {
    FfmpegVideoFrame(FfmpegVideoFrame),
    BrawVideoFrame(BrawVideoFrame)
}

#[enum_delegate::register]
pub trait AudioFrameInterface {
    fn timestamp_us(&self) -> Option<i64>;
    fn buffer_size(&self) -> u32;
}

#[enum_delegate::implement(AudioFrameInterface)]
pub enum AudioFrame {
    FfmpegAudioFrame(FfmpegAudioFrame)
}

pub enum Frame {
    Video(VideoFrame),
    Audio(AudioFrame),
    Other
}
