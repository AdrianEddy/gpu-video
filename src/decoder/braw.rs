// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2025 Adrian <adrian.eddy at gmail>

use super::*;
use crate::types::VideoProcessingError;
use crate::frame::braw::BrawVideoFrame;
use crate::util::select_custom_option;
use std::sync::LazyLock;
use parking_lot::Mutex;
use core::ffi::c_void;
use std::hash::Hash;
use crate::buffer_pool::BufferPool;
use std::sync::Arc;
use ::braw::*;


struct GlobalFactory(Factory);
unsafe impl Send for GlobalFactory {}
unsafe impl Sync for GlobalFactory {}

struct StreamInfo {
    info: Stream,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct BrawTypeAndFormat {
    pub(crate) kind: BlackmagicRawResourceType,
    pub(crate) pixel_format: BlackmagicRawResourceFormat,
    pub(crate) size_bytes: Option<usize>,
}
impl Hash for BrawTypeAndFormat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u32(self.kind as u32);
        state.write_u32(self.pixel_format as u32);
        state.write_usize(self.size_bytes.unwrap_or(0));
    }
}

pub(crate) struct BrawRawResource {
    pub(crate) kind: BlackmagicRawResourceType,
    pub(crate) resmgr: BlackmagicRawResourceManager,
    pub(crate) context: Option<*mut c_void>,
    pub(crate) queue: Option<*mut c_void>,
    pub(crate) data: *mut c_void,
    pub(crate) size: usize,
}

pub(crate) struct BrawResourceFactory {
    context: Option<*mut c_void>,
    queue: Option<*mut c_void>,
    resmgr: BlackmagicRawResourceManager,
}
impl BufferFactory<BrawRawResource, BrawTypeAndFormat> for BrawResourceFactory {
    fn create(&mut self, width: u32, height: u32, stride: usize, format: &BrawTypeAndFormat) -> FrameBuffer<BrawRawResource, BrawTypeAndFormat> { // TODO: result
        log::debug!("Creating BRAW resource buffer: {:?}", format);
        let mut img = self.resmgr.create_resource(self.context.unwrap_or(std::ptr::null_mut()), self.queue.unwrap_or(std::ptr::null_mut()), format.size_bytes.unwrap_or(0) as u32, format.kind, BlackmagicRawResourceUsage::ReadCPUWriteCPU).unwrap();
        if img.is_null() {
            panic!("Failed to create BRAW resource buffer");
        }
        FrameBuffer {
            width,
            height,
            stride,
            format: *format,
            inner: BrawRawResource {
                kind: format.kind,
                resmgr: self.resmgr.clone(),
                context: self.context,
                queue: self.queue,
                data: img,
                size: format.size_bytes.unwrap_or(0),
            }
        }
    }

    fn free(&mut self, buffer: FrameBuffer<BrawRawResource, BrawTypeAndFormat>) {
        log::debug!("Dropping BRAW resource buffer: {:?}", buffer);
        self.resmgr.release_resource(buffer.inner.context.unwrap_or(std::ptr::null_mut()), buffer.inner.queue.unwrap_or(std::ptr::null_mut()), buffer.inner.data, buffer.inner.kind).unwrap(); // TODO: result
    }
}

pub struct BrawDecoder {
    frame_rate: f64,
    frame_count: u64,

    current_frame: u64,

    open_options: DecoderOptions,

    stream_state: Vec<StreamInfo>,

    resolution_scale: Option<BlackmagicRawResolutionScale>,
    resource_format: Option<BlackmagicRawResourceFormat>,

    // Drop order is important here
    buffer_pool: Arc<BufferPool<BrawRawResource, BrawTypeAndFormat, BrawResourceFactory>>,
    clip: BlackmagicRawClip,
    codec: BlackmagicRaw,
    resource_manager: BlackmagicRawResourceManager,
    device: Option<BlackmagicRawPipelineDevice>,
}

impl Drop for BrawDecoder {
    fn drop(&mut self) {
        let _ = self.codec.flush_jobs();
    }
}

impl DecoderInterface for BrawDecoder {
    fn streams(&mut self) -> Vec<&mut Stream> {
        self.stream_state.iter_mut().map(|x| &mut x.info).collect()
    }

    fn seek(&mut self, timestamp_us: i64) -> Result<bool, VideoProcessingError> {
        self.current_frame = ((timestamp_us as f64 * self.frame_rate / 1_000_000.0).round() as i64)
            .min(self.frame_count as i64 - 1)
            .max(0) as u64;
        Ok(true)
    }

    fn get_video_info(&self) -> Result<VideoInfo, VideoProcessingError> {
        Ok(VideoInfo {
            duration_ms: self.frame_count as f64 * 1000.0 / self.frame_rate,
            frame_count: self.frame_count as usize,
            fps:         self.frame_rate,
            width:       self.clip.width()?,
            height:      self.clip.height()?,
            bitrate:     0.0,
        })
    }

    fn next_frame(&mut self) -> Result<Option<Frame>, VideoProcessingError> {
        if self.current_frame >= self.frame_count {
            return Ok(None);
        }
        pollster::block_on(async {
            let frame = self.clip.read_frame(self.current_frame).await?;

            if let Some(scale) = self.resolution_scale { frame.set_resolution_scale(scale)?; }
            if let Some(format) = self.resource_format { frame.set_resource_format(format)?; }

            let data = frame.decode_and_process(None, None).await?; // TODO handle errors

            let timestamp_us = self.current_frame as i64 * 1_000_000 / self.frame_rate as i64;

            self.current_frame += 1;
            Ok(Some(Frame::Video(BrawVideoFrame {
                timestamp_us,
                width: data.width()?,
                height: data.height()?,
                format: data.resource_format()?,
                buffer_pool: self.buffer_pool.clone(),
                resource_manager: self.resource_manager.clone(),
                frame: data,
                cpu_frame: None,
            }.into())))
        })
    }
}

impl BrawDecoder {
    pub fn new(mut path: &str, options: DecoderOptions) -> Result<Self, VideoProcessingError> {
        static LIBRARY: LazyLock<Mutex<GlobalFactory>> = LazyLock::new(|| {
            Mutex::new(GlobalFactory(Factory::load_from(default_library_name()).unwrap()))
        });
        use std::sync::Arc;
        use std::borrow::Cow;

        let (codec, device, context, queue) = {
            let factory = LIBRARY.lock();
            let codec = factory.0.create_codec()?;
            let mut config = codec.configuration()?;

            let mut device = None;
            let mut context = None;
            let mut queue = None;

            if let Some(gpu_index) = options.gpu_index {
                'p: for p in factory.0.pipeline_iter(BlackmagicRawInterop::None)? {
                    log::debug!("BRAW pipeline: {}, pipeline={:?}, interop={:?}", p.name, p.pipeline, p.interop);
                    if let Ok(piter) = factory.0.pipeline_device_iter(p.pipeline, p.interop) {
                        for dev in piter.skip(gpu_index) {
                            if let Ok(created_device) = dev.create_device() {
                                log::debug!("BRAW created device: {}, index={}, pipeline={:?}, interop={:?}, max_texture={:?}",
                                    created_device.name()?,
                                    created_device.index()?,
                                    created_device.pipeline_name()?,
                                    created_device.interop()?,
                                    created_device.maximum_texture_size()?
                                );
                                let (_, context2, queue2) = created_device.pipeline()?;
                                context = Some(context2);
                                queue = Some(queue2);
                                config.set_from_device(created_device.clone())?;

                                // pollster::block_on(codec.prepare_pipeline_for_device(created_device.clone())?);

                                device = Some(created_device);
                                break 'p;
                            } else {
                                log::warn!("Failed to create BRAW device for {:?}", dev.pipeline);
                            }
                        }
                    }
                }
            }
            (codec, device, context, queue)
        };

        let resmgr = codec.configuration_ex()?.resource_manager()?;

        let clip = codec.open_clip(path)?;

        let mut stream_state = Vec::new();

        let fps = clip.frame_rate()?;
        let fps_rational = Rational((fps * 1000.0) as i32, 1000); // TODO: guess rational better

        stream_state.push(StreamInfo {
            info: Stream {
                stream_type: StreamType::Video,
                index: 0,
                avg_frame_rate: fps_rational,
                rate:           fps_rational,
                time_base:      fps_rational.invert(),

                decode: true,
            }
        });

        let buffer_factory = BrawResourceFactory {
            resmgr: resmgr.clone(),
            context: context,
            queue: queue
        };

        let buffer_pool = Arc::new(BufferPool::new(4, buffer_factory));

        let resolution_scale = if let Some(value) = select_custom_option(&options.custom_options, &["braw.decode_resolution", "decode_resolution"]) {
            match parse_resolution_scale(value) {
                Some(scale) => Some(scale),
                None => { log::warn!("BRAW: ignoring unknown decode_resolution '{value}'"); None }
            }
        } else {
            None
        };
        let resource_format = if let Some(value) = select_custom_option(&options.custom_options, &["braw.output_format", "output_format"]) {
            match parse_resource_format(value) {
                Some(format) => Some(format),
                None => { log::warn!("BRAW: ignoring unknown output_format '{value}'"); None }
            }
        } else {
            None
        };

        Ok(Self {
            codec: codec,
            clip: clip.clone(),
            device: device.clone(),
            resource_manager: resmgr,
            buffer_pool,

            frame_rate: clip.frame_rate()? as f64,
            frame_count: clip.frame_count()?,
            current_frame: 0,

            open_options: options,

            stream_state,
            resolution_scale,
            resource_format,
        })
    }
}

fn parse_resolution_scale(value: &str) -> Option<BlackmagicRawResolutionScale> {
    match value.to_ascii_lowercase().trim() {
        "full"    | "1"   => Some(BlackmagicRawResolutionScale::Full),
        "half"    | "1/2" => Some(BlackmagicRawResolutionScale::Half),
        "quarter" | "1/4" => Some(BlackmagicRawResolutionScale::Quarter),
        "eighth"  | "1/8" => Some(BlackmagicRawResolutionScale::Eighth),
        _ => None,
    }
}

fn parse_resource_format(value: &str) -> Option<BlackmagicRawResourceFormat> {
    match value.to_ascii_lowercase().trim() {
        "rgba8"  => Some(BlackmagicRawResourceFormat::RGBAU8),
        "bgra8"  => Some(BlackmagicRawResourceFormat::BGRAU8),
        "rgb16"  => Some(BlackmagicRawResourceFormat::RGBU16),
        "rgba16" => Some(BlackmagicRawResourceFormat::RGBAU16),
        "bgra16" => Some(BlackmagicRawResourceFormat::BGRAU16),
        "rgb16_planar" => Some(BlackmagicRawResourceFormat::RGBU16Planar),
        "rgbf32"  => Some(BlackmagicRawResourceFormat::RGBF32),
        "rgbaf32" => Some(BlackmagicRawResourceFormat::RGBAF32),
        "bgraf32" => Some(BlackmagicRawResourceFormat::BGRAF32),
        "rgbf32_planar" => Some(BlackmagicRawResourceFormat::RGBF32Planar),
        "rgbf16"  => Some(BlackmagicRawResourceFormat::RGBF16),
        "rgbaf16" => Some(BlackmagicRawResourceFormat::RGBAF16),
        "bgraf16" => Some(BlackmagicRawResourceFormat::BGRAF16),
        "rgbf16_planar" => Some(BlackmagicRawResourceFormat::RGBF16Planar),
        _ => None,
    }
}
