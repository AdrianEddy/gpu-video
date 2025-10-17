// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

use thiserror::Error;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PixelFormat {
    Unknown,
    AYUV64LE,

    NV12, NV21,
    NV16,
    NV24, NV42,
    P010LE, P016LE,
    P210LE, P216LE,
    P410LE, P416LE,
    //RGB32,
    //RGB48BE,
    RgbU8,  RgbU16,  RgbF16,  RgbF32,
    RgbaU8, RgbaU16, RgbaF16, RgbaF32,
    BgrU8,  BgrU16, BgrF16, BgrF32,
    BgraU8, BgraU16, BgraF16, BgraF32,
    //BGRA,
    //RGBA64BE,
    YUV420P, YUV420P10LE, YUV420P12LE, YUV420P14LE, YUV420P16LE,
    YUV422P, YUV422P10LE, YUV422P12LE, YUV422P14LE, YUV422P16LE,
    YUV444P, YUV444P10LE, YUV444P12LE, YUV444P14LE, YUV444P16LE,

    UYVY422
}

#[derive(Debug, Copy, Clone)]
pub enum ColorRange {
    Full,
    Limited
}

#[derive(Debug, Copy, Clone)]
pub enum StreamType {
    Video,
    Audio,
    Subtitle,
    Metadata,
    Other
}

#[derive(Debug, Clone)]
pub struct Stream {
    pub stream_type: StreamType,
    pub index: usize,
    pub time_base: Rational,
    pub avg_frame_rate: Rational,
    pub rate: Rational,

    pub decode: bool,
}

#[derive(Debug)]
pub enum HWTexture {
    D3D11 { resource: *mut std::ffi::c_void }, // ID3D11Texture2D*
    DXVA2 { resource: *mut std::ffi::c_void }, // IDirect3DSurface9*
    QSV   { resource: *mut std::ffi::c_void }, // mfxFrameSurface1*
    VAAPI { resource: u32 }, // VASurfaceID
    VDPAU { resource: u32 }, // VdpVideoSurface
    CUDA  { resource: *mut std::ffi::c_void }, // CuDevicePtr
    OpenCL { memory: *mut std::ffi::c_void }, // cl_mem
    VideoToolbox { resource: *mut std::ffi::c_void }, // MTLTexture*
    MetalTexture { texture: *mut std::ffi::c_void }, // MTLTexture*
    MetalBuffer  { buffer: *mut std::ffi::c_void }, // MTLBuffer*
}

#[derive(Debug, Clone, Default)]
pub struct VideoInfo {
    pub duration_ms: f64,
    pub frame_count: usize,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
    pub bitrate: f64, // in Mbps
}

#[derive(Debug, Clone, Copy)]
pub struct Rational(pub i32, pub i32);
impl Rational {
    pub fn invert(&self) -> Self { Self(self.1, self.0) }
    pub fn as_f32(&self) -> f32 { self.1 as f32 / self.0 as f32 }
}
impl From<f32> for Rational {
    fn from(value: f32) -> Self {
        todo!()
    }
}


#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VideoCodec {
    H264, H265, AV1, ProRes, DNxHR, CineForm, PNG, EXR
}
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AudioCodec {
    AAC, PCM
}
#[derive(Copy, Clone, Debug)]
pub enum Bitrate {
    Constant(f64), // in Mbps
    Variable((f64, f64)), // min, max in Mbps
    QScale(i32)
}
pub enum StreamParams {
    Video {
        width: u32,
        height: u32,
        format: Option<crate::types::PixelFormat>,
        bitrate: Bitrate,
        codec: VideoCodec,
        use_gpu: bool,
        frame_rate: Rational,
        time_base: Option<Rational>,
        custom_options: HashMap<String, String>,

        color_range: ColorRange,
        // color_space: Option<ColorSpace>,
        // color_trc: Option<ColorTrc>,
        // color_primaries: Option<ColorPrimaries>,
        // aspect_ratio: Option<(u32, u32)>,
    },
    Audio {
        codec: AudioCodec,
        bitrate: Bitrate,
        sample_rate: u32,
        time_base: Option<(u32, u32)>,
        custom_options: HashMap<String, String>,
    }
}


#[derive(Error, Debug)]
pub enum VideoProcessingError {
    #[error("Encoder not found")]
    EncoderNotFound,
    #[error("Decoder not found")]
    DecoderNotFound,
    #[error("No supported formats")]
    NoSupportedFormats,
    #[error("No output context")]
    NoOutputContext,
    #[error("Encoder converter is null")]
    EncoderConverterEmpty,
    #[error("Video stream was not found")]
    VideoStreamNotFound,
    #[error("Converter is null")]
    ConverterEmpty,
    #[error("Frame is null")]
    FrameEmpty,
    #[error("No GPU decoding device")]
    NoGPUDecodingDevice,
    #[error("No hardware transfer formats")]
    NoHWTransferFormats,
    #[error("Error transferring frame from the GPU: {0:?}")] // , ffmpeg_next::Error::Other { errno: .0 }
    FromHWTransferError(i32),
    #[error("Error transferring frame to the GPU: {0:?}")] // , ffmpeg_next::Error::Other { errno: .0 }
    ToHWTransferError(i32),
    #[error("Unable to create HW devices context")]
    CannotCreateGPUDecoding,
    #[error("Empty hw frames context")]
    NoFramesContext,
    #[error("GPU decoding failed, please try again.")]
    GPUDecodingFailed,
    #[error("Error getting HW transfer buffer to the GPU: {0:?}")] // , ffmpeg_next::Error::Other { errno: .0 }
    ToHWBufferError(i32),
    #[error("Pixel format {format:?} is not supported. Supported ones: {supported:?}")]
    PixelFormatNotSupported { format: PixelFormat, supported: Vec<PixelFormat> },
    #[error("Unknown pixel format: {0:?}")]
    UnknownPixelFormat(PixelFormat),

    #[cfg(feature = "ffmpeg")]
    #[error("ffmpeg error: {0:?}")]
    FfmpegError(#[from] ffmpeg_next::Error),

    #[cfg(feature = "braw")]
    #[error("BRAW error: {0:?}")]
    BrawError(#[from] ::braw::BrawError),
    #[cfg(feature = "r3d")]
    #[error("R3D error: {0:?}")]
    R3DError(#[from] ::r3d_rs::RedError),
}
