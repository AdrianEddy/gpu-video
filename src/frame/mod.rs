// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

#[cfg(feature = "ffmpeg")] pub(crate) mod ffmpeg; #[cfg(feature = "ffmpeg")] use ffmpeg::*;
#[cfg(feature = "braw")]   pub(crate) mod braw;   #[cfg(feature = "braw")]   use braw::*;
#[cfg(feature = "r3d")]    pub(crate) mod r3d;    #[cfg(feature = "r3d")]    use r3d::*;
use crate::types::*;

pub struct TextureDescription {
    pub texture: HWTexture,
}

#[enum_dispatch::enum_dispatch(VideoFrame)]
pub trait VideoFrameInterface {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn timestamp_us(&self) -> Option<i64>;
    fn format(&self) -> PixelFormat;
    fn get_cpu_buffers(&mut self) -> Result<Vec<&mut [u8]>, crate::VideoProcessingError>;
    fn get_gpu_texture(&mut self, plane: usize) -> Option<TextureDescription>;
    fn color_range(&self) -> Option<ColorRange>;
    fn color_space(&self) -> Option<ColorSpace>;
}

#[enum_dispatch::enum_dispatch]
pub enum VideoFrame {
    Unknown(NullVideoFrame),
    #[cfg(feature = "ffmpeg")]
    FfmpegVideoFrame(FfmpegVideoFrame),
    #[cfg(feature = "braw")]
    BrawVideoFrame(BrawVideoFrame),
    #[cfg(feature = "r3d")]
    R3dVideoFrame(R3dVideoFrame)
}


#[enum_dispatch::enum_dispatch(AudioFrame)]
pub trait AudioFrameInterface {
    fn timestamp_us(&self) -> Option<i64>;
    fn buffer_size(&self) -> u32;
}

#[enum_dispatch::enum_dispatch]
pub enum AudioFrame {
    Unknown(NullAudioFrame),
    #[cfg(feature = "ffmpeg")]
    FfmpegAudioFrame(FfmpegAudioFrame)
}

pub enum Frame {
    Video(VideoFrame),
    Audio(AudioFrame),
    Other
}



pub struct NullAudioFrame;
impl AudioFrameInterface for NullAudioFrame {
    fn timestamp_us(&self) -> Option<i64> { None }
    fn buffer_size(&self) -> u32 { 0 }
}
pub struct NullVideoFrame;
impl VideoFrameInterface for NullVideoFrame {
    fn width(&self) -> u32 { 0 }
    fn height(&self) -> u32 { 0 }
    fn timestamp_us(&self) -> Option<i64> { None }
    fn format(&self) -> PixelFormat { PixelFormat::Unknown }
    fn get_cpu_buffers(&mut self) -> Result<Vec<&mut [u8]>, crate::VideoProcessingError> {
        Err(crate::VideoProcessingError::FrameEmpty)
    }
    fn get_gpu_texture(&mut self, _plane: usize) -> Option<TextureDescription> {
        None
    }
    fn color_range(&self) -> Option<ColorRange> { None }
    fn color_space(&self) -> Option<ColorSpace> { None }
}