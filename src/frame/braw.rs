// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2025 Adrian <adrian.eddy at gmail>

use super::*;
use crate::decoder::braw::*;
use ::braw::*;
use crate::buffer_pool::*;
use std::sync::Arc;
use core::ffi::c_void;

pub struct BrawVideoFrame {
    pub(crate) timestamp_us: i64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: BlackmagicRawResourceFormat,
    pub(crate) frame: BlackmagicRawProcessedImage,
    pub(crate) resource_manager: BlackmagicRawResourceManager,
    pub(crate) buffer_pool: Arc<BufferPool<BrawRawResource, BrawTypeAndFormat, BrawResourceFactory>>,
    pub(crate) cpu_frame: Option<PooledFrame<BrawRawResource, BrawTypeAndFormat, BrawResourceFactory>>,
}

impl VideoFrameInterface for BrawVideoFrame {
    fn width(&self)  -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn timestamp_us(&self) -> Option<i64> { Some(self.timestamp_us) }

    fn format(&self) -> PixelFormat {
        match self.format {
            BlackmagicRawResourceFormat::RGBAU8  => PixelFormat::RgbaU8,
            BlackmagicRawResourceFormat::BGRAU8  => PixelFormat::BgraU8,
            BlackmagicRawResourceFormat::RGBU16  => PixelFormat::RgbU16,
            BlackmagicRawResourceFormat::RGBAU16 => PixelFormat::RgbaU16,
            BlackmagicRawResourceFormat::BGRAU16 => PixelFormat::BgraU16,
            BlackmagicRawResourceFormat::RGBF32  => PixelFormat::RgbF32,
            BlackmagicRawResourceFormat::RGBAF32 => PixelFormat::RgbaF32,
            BlackmagicRawResourceFormat::BGRAF32 => PixelFormat::BgraF32,
            BlackmagicRawResourceFormat::RGBF16  => PixelFormat::RgbF16,
            BlackmagicRawResourceFormat::RGBAF16 => PixelFormat::RgbaF16,
            BlackmagicRawResourceFormat::BGRAF16 => PixelFormat::BgraF16,
            // BlackmagicRawResourceFormat::RGBU16Planar =>
            // BlackmagicRawResourceFormat::RGBF32Planar =>
            // BlackmagicRawResourceFormat::RGBF16Planar =>
            f => {
                log::error!("Unknown pixel format: {f:?}");
                PixelFormat::Unknown
            }
        }
    }

    fn get_cpu_buffers(&mut self) -> Result<Vec<&mut [u8]>, crate::VideoProcessingError> {
        match self.frame.resource_type()? {
            BlackmagicRawResourceType::BufferMetal |
            BlackmagicRawResourceType::BufferCUDA |
            BlackmagicRawResourceType::BufferOpenCL => {
                let data_size = self.frame.resource_size_bytes()? as usize;

                self.cpu_frame = Some(self.buffer_pool.get(self.width, self.height, 0, BrawTypeAndFormat {
                    kind: BlackmagicRawResourceType::BufferCPU,
                    pixel_format: self.format,
                    size_bytes: Some(data_size)
                }));
                let cpu_frame2 = self.cpu_frame.as_ref().unwrap().buffer();

                let src = self.frame.resource_gpu()?;
                let (context, queue) = self.frame.resource_context_and_command_queue()?;

                self.resource_manager.copy_resource(
                    context,
                    queue,
                    src.1 as *mut c_void,
                    src.0,
                    cpu_frame2.inner.data,
                    cpu_frame2.inner.kind,
                    data_size as u32,
                    false // copy_async
                ).unwrap();

                //let host_ptr = self.resource_manager.resource_host_pointer(self.context.unwrap_or(std::ptr::null_mut()), self.queue.unwrap_or(std::ptr::null_mut()), cpu_frame2.inner.data, cpu_frame2.inner.kind)?;
                Ok(vec![
                    unsafe {
                        std::slice::from_raw_parts_mut(cpu_frame2.inner.data as *mut u8, data_size)
                    }
                ])
            }
            BlackmagicRawResourceType::BufferCPU => {
                let resource = self.frame.resource_cpu()?;
                Ok(vec![ unsafe { std::slice::from_raw_parts_mut(resource.as_ptr() as *mut u8, resource.len()) } ])
            }
            _ => {
                log::error!("Unknown resource type: {:?}", self.frame.resource_type());
                Err(VideoProcessingError::NoSupportedFormats)
            }
        }
    }

    fn get_gpu_texture(&mut self, plane: usize) -> Option<TextureDescription> { // TODO: result
        match self.frame.resource_type().ok()? {
            BlackmagicRawResourceType::BufferMetal => {
                let (_kind, ptr) = self.frame.resource_gpu().ok()?;
                Some(TextureDescription {
                    texture: HWTexture::MetalTexture { texture: ptr as *mut _ } // MTLTexture*
                })
            }
            BlackmagicRawResourceType::BufferCUDA => {
                let (_kind, ptr) = self.frame.resource_gpu().ok()?;
                Some(TextureDescription {
                    texture: HWTexture::CUDA { resource: ptr as *mut _ } // CuDevicePtr
                })
            }
            BlackmagicRawResourceType::BufferOpenCL => {
                let (_kind, ptr) = self.frame.resource_gpu().ok()?;
                Some(TextureDescription {
                    texture: HWTexture::OpenCL { memory: ptr as *mut _ } // cl_mem
                })
            }
            BlackmagicRawResourceType::BufferCPU => {
                // TODO: upload to GPU
                //let resource = self.frame.resource_cpu()?;
                //Ok(vec![ unsafe { std::slice::from_raw_parts_mut(resource.as_ptr() as *mut u8, resource.len()) } ])
                None
            }
            _ => {
                log::error!("Unknown resource type: {:?}", self.frame.resource_type());
                None
            }
        }
    }
}
