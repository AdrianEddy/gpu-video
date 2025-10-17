
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2025 Adrian <adrian.eddy at gmail>

use super::*;
use crate::decoder::r3d::*;
use crate::buffer_pool::*;
use r3d_rs::*;
use std::sync::Arc;

pub struct R3dVideoFrame {
	pub(crate) timestamp_us: i64,
	pub(crate) width: u32,
	pub(crate) height: u32,
	pub(crate) pixel_type: VideoPixelType,
	pub(crate) buffer_pool: Arc<BufferPool<AlignedBuffer, R3dTypeAndFormat, R3dBufferFactory>>,
	pub(crate) cpu_frame: Option<PooledFrame<AlignedBuffer, R3dTypeAndFormat, R3dBufferFactory>>,
}

impl VideoFrameInterface for R3dVideoFrame {
	fn width(&self) -> u32 { self.width }
	fn height(&self) -> u32 { self.height }
	fn timestamp_us(&self) -> Option<i64> { Some(self.timestamp_us) }

	fn format(&self) -> PixelFormat {
		match self.pixel_type {
			VideoPixelType::Bgra8bitInterleaved => PixelFormat::BgraU8,
			VideoPixelType::Bgr8bitInterleaved => PixelFormat::BgrU8,
			VideoPixelType::Rgb16bitInterleaved => PixelFormat::RgbU16,
			VideoPixelType::RgbHalfFloatInterleaved => PixelFormat::RgbF16,
			VideoPixelType::RgbHalfFloatAcesInt => PixelFormat::RgbF16,
			VideoPixelType::Rgb16bitPlanar => PixelFormat::RgbU16,
			VideoPixelType::Dpx10bitMethodB => PixelFormat::Unknown, // TODO: implement later
		}
	}

	fn get_cpu_buffers(&mut self) -> Result<Vec<&mut [u8]>, crate::VideoProcessingError> {
		if let Some(ref mut pooled) = self.cpu_frame {
            let buf = pooled.buffer_mut();
			let len = buf.inner.len();
			let ptr = buf.inner.ptr as *mut u8;
			unsafe {
				Ok(vec![ std::slice::from_raw_parts_mut(ptr, len) ])
			}
		} else {
			Err(crate::VideoProcessingError::FrameEmpty)
		}
	}

	fn get_gpu_texture(&mut self, _plane: usize) -> Option<TextureDescription> {
		// CPU path only for now. In the future we can expose CUDA/OpenCL resources
		None
	}
}
