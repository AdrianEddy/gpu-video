// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

use thiserror::Error;

#[derive(Debug)]
pub enum PixelFormat {
    Unknown,
    AYUV64LE,

    NV12, NV21,
    NV16,
    NV24, NV42,
    P010LE, P016LE,
    P210LE, P216LE,
    P410LE, P416LE,
    RGB32,
    RGB48BE,
    RGBA,
    BGRA,
    RGBA64BE,
    YUV420P, YUV420P10LE, YUV420P12LE, YUV420P14LE, YUV420P16LE,
    YUV422P, YUV422P10LE, YUV422P12LE, YUV422P14LE, YUV422P16LE,
    YUV444P, YUV444P10LE, YUV444P12LE, YUV444P14LE, YUV444P16LE,

    UYVY422
}

#[derive(Debug)]
pub enum HWTexture {
    D3D11 { resource: *mut std::ffi::c_void }, // ID3D11Texture2D*
    DXVA2 { resource: *mut std::ffi::c_void }, // IDirect3DSurface9*
    QSV   { resource: *mut std::ffi::c_void }, // mfxFrameSurface1*
    VAAPI { resource: u32 }, // VASurfaceID
    VDPAU { resource: u32 }, // VdpVideoSurface
    CUDA  { resource: *mut std::ffi::c_void }, // CuDevicePtr
    VideoToolbox { resource: *mut std::ffi::c_void }, // MTLTexture*
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
    #[error("ffmpeg error: {0:?}")]
    InternalError(#[from] ffmpeg_next::Error),
}
