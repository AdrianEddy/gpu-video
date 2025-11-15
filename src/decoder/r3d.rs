// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2025 Adrian <adrian.eddy at gmail>

use super::*;
use crate::types::VideoProcessingError;
use crate::frame::r3d::R3dVideoFrame;
use crate::util::select_custom_option;
use crate::buffer_pool::{BufferFactory, BufferPool, FrameBuffer};
use std::hash::Hash;
use std::sync::Arc;
use std::sync::OnceLock;
use parking_lot::Mutex;

use r3d_rs::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct R3dTypeAndFormat {
    pub(crate) mode: VideoDecodeMode,
    pub(crate) pixel_type: VideoPixelType,
    pub(crate) size_bytes: Option<usize>,
}
impl Hash for R3dTypeAndFormat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i32(self.mode as i32);
        state.write_i32(self.pixel_type as i32);
        state.write_usize(self.size_bytes.unwrap_or(0));
    }
}

pub(crate) struct R3dBufferFactory {
    size_bytes: usize,
}
impl BufferFactory<AlignedBuffer, R3dTypeAndFormat> for R3dBufferFactory {
    fn create(&mut self, width: u32, height: u32, stride: usize, format: &R3dTypeAndFormat) -> Result<FrameBuffer<AlignedBuffer, R3dTypeAndFormat>, VideoProcessingError> {
        let size = format.size_bytes.unwrap_or(self.size_bytes);
        let buf = AlignedBuffer::new(size, 16)?;
        Ok(FrameBuffer {
            width,
            height,
            stride,
            format: *format,
            inner: buf,
        })
    }

    fn free(&mut self, _buffer: FrameBuffer<AlignedBuffer, R3dTypeAndFormat>) -> Result<(), VideoProcessingError> {
        // Dropping the AlignedBuffer will free memory automatically
        Ok(())
    }
}

enum SdkHolder {
    Initialized(r3d_rs::Sdk),
    Dummy,
}

pub struct R3dDecoder {
    frame_rate: f64,
    frame_count: u64,

    current_frame: u64,

    open_options: DecoderOptions,

    stream_state: Vec<Stream>,

    // Pool of CPU-aligned frame buffers
    buffer_pool: Arc<BufferPool<AlignedBuffer, R3dTypeAndFormat, R3dBufferFactory>>,

    clip: Clip,
    decoder: r3d_rs::R3dDecoder,

    // Selected decode settings
    mode: VideoDecodeMode,
    pixel_type: VideoPixelType,
    image_settings: ImageProcessingSettings,
}

impl DecoderInterface for R3dDecoder {
    fn streams(&mut self) -> Vec<&mut Stream> {
        self.stream_state.iter_mut().collect()
    }

    fn seek(&mut self, timestamp_us: i64) -> Result<bool, VideoProcessingError> {
        self.current_frame = ((timestamp_us as f64 * self.frame_rate / 1_000_000.0).round() as i64)
            .min(self.frame_count as i64 - 1)
            .max(0) as u64;
        Ok(true)
    }

    fn get_video_info(&self) -> Result<VideoInfo, VideoProcessingError> {
        let mut metadata = HashMap::new();
        for (k, v) in self.clip.metadata_iter() {
            metadata.insert(k.to_string(), format!("{v}"));
        }

        Ok(VideoInfo {
            duration_ms: self.frame_count as f64 * 1000.0 / self.frame_rate,
            frame_count: self.frame_count as usize,
            fps: self.frame_rate,
            width: self.clip.width() as u32,
            height: self.clip.height() as u32,
            bitrate: 0.0,

            created_at:  None, // TODO?
            rotation:    0, // TODO?
            metadata:    metadata,
        })
    }

    fn next_frame(&mut self) -> Result<Option<Frame>, VideoProcessingError> {
        if self.current_frame >= self.frame_count { return Ok(None); }

        let (width, height) = scaled_dims(self.clip.width() as u32, self.clip.height() as u32, &self.mode);
        let bpp = bytes_per_pixel(self.pixel_type);
        let stride = width as usize * bpp;

        let size_needed = self.clip.calculate_buffer_size(&self.mode, &self.pixel_type)?;

        let pooled = self.buffer_pool.get(width, height, stride, R3dTypeAndFormat {
            mode: self.mode,
            pixel_type: self.pixel_type,
            size_bytes: Some(size_needed),
        })?;
        let buf_ptr = pooled.buffer().inner.ptr;
        let buf_len = pooled.buffer().inner.len();

        // Build and submit the job
        let mut job = R3dDecodeJob::new()?;
        job.set_clip(&self.clip);
        job.set_mode(self.mode);
        job.set_pixel_type(self.pixel_type);
        job.set_video_track_no(0);
        job.set_video_frame_no(self.current_frame as usize);
        job.set_image_processing(&self.image_settings);
        job.set_output_buffer(buf_ptr, buf_len);
        job.allocate_frame_metadata();

        let job = pollster::block_on(self.decoder.decode(job)?)?; // Block until done

        let timestamp_us = self.current_frame as i64 * 1_000_000 / self.frame_rate as i64;
        self.current_frame += 1;

        let mut metadata = HashMap::new();

        if let Ok(meta) = job.metadata() {
            for (k, v) in meta.iter() {
                metadata.insert(k, v);
            }
        }

        Ok(Some(Frame::Video(R3dVideoFrame {
            timestamp_us,
            width,
            height,
            metadata,
            pixel_type: self.pixel_type,
            cpu_frame: Some(pooled),
        }.into())))
    }
}

impl R3dDecoder {
    pub fn new<'a>(input: IoType<'a>, filename: Option<&str>, options: DecoderOptions) -> Result<Self, VideoProcessingError> {
        static LIBRARY: OnceLock<Result<Mutex<SdkHolder>, ::r3d_rs::RedError>> = OnceLock::new();
        static CUSTOM_IO: OnceLock<Mutex<CustomIO>> = OnceLock::new();

        let lib = LIBRARY.get_or_init(|| {
            let mut flags = InitializeFlags::R3DDecoder | InitializeFlags::Cuda | InitializeFlags::OpenCL;
            if cfg!(target_os = "macos") {
                flags |= InitializeFlags::Metal;
            }

            let check = if cfg!(target_os = "windows") {
                ("win", "REDCuda-x64.dll")
            } else if cfg!(target_os = "macos") {
                ("mac", "REDR3D.dylib")
            } else {
                ("linux", "REDR3D-x64.so")
            };

            let mut sdk_path = ".".to_string();

            let candidates = vec![
                ".".to_string(),
                std::env::var("R3DSDK_DIR").unwrap_or_default(),
                crate::util::select_custom_option(&options.custom_options, &["r3d.sdk_path", "R3DSDK_DIR"]).unwrap_or_default().to_string(),
            ];
            for candidate in candidates {
                let mut path1 = std::path::Path::new(&candidate).join("Redistributable").join(&check.0).join(&check.1);
                let mut path2 = std::path::Path::new(&candidate).join(&check.1);
                if path1.exists() {
                    path1.pop();
                    sdk_path = path1.to_string_lossy().to_string();
                    break;
                }
                if path2.exists() {
                    path2.pop();
                    sdk_path = path2.to_string_lossy().to_string();
                    break;
                }
            }
            sdk_path = sdk_path.replace("\\", "/").replace("//", "/");
            if cfg!(target_os = "windows") {
                sdk_path = sdk_path.replace("/", "\\");
            }
            log::debug!("Trying to load R3D SDK from {sdk_path}");

            if Sdk::version().contains("R3DSDK") {
                log::warn!("R3D SDK already initialized!");
                return Ok(Mutex::new(SdkHolder::Dummy));
            }

            for _ in 0..3 {
                match Sdk::initialize(&sdk_path, flags) {
                    Ok(sdk) => {
                        return Ok(Mutex::new(SdkHolder::Initialized(sdk)));
                    },
                    Err(::r3d_rs::RedError::RedCudaLibraryNotFound) if flags.contains(InitializeFlags::Cuda) => {
                        flags &= !InitializeFlags::Cuda;
                    },
                    Err(::r3d_rs::RedError::RedOpenCLLibraryNotFound) if flags.contains(InitializeFlags::OpenCL) => {
                        flags &= !InitializeFlags::OpenCL;
                    },
                    Err(::r3d_rs::RedError::RedMetalLibraryNotFound) if flags.contains(InitializeFlags::Metal) => {
                        flags &= !InitializeFlags::Metal;
                    }
                    Err(e) => {
                        log::error!("Failed to initialize R3D SDK: {e:?}");
                        return Err(e)
                    }
                }
            }
            Err(::r3d_rs::RedError::UnableToLoadLibrary)
        });
        let lib2 = match lib {
            Ok(mutex) => mutex,
            Err(e) => { return Err(e.clone().into()); }
        };
        let _sdk = lib2.lock(); // TODO this lock is probably too excessive

        match input {
            IoType::Bytes(_) |
            IoType::ReadSeekStream { .. } |
            IoType::ReadWriteSeekStream { .. }  => {
                // Install global custom IO
                let _io = CUSTOM_IO.get_or_init(move || {
                    Mutex::new(CustomIO::install(Box::new(StreamIo::with_filesystem_fallback())))
                });
            }
            IoType::FileList(ref map) => {
                if map.values().any(|v| matches!(v, IoType::Bytes(_) | IoType::ReadSeekStream { .. } | IoType::ReadWriteSeekStream { .. })) {
                    // Install global custom IO
                    let _io = CUSTOM_IO.get_or_init(move || {
                        Mutex::new(CustomIO::install(Box::new(StreamIo::with_filesystem_fallback())))
                    });
                }
            }
            _ => { }
        }

        // Open clip
        let clip = match input {
            IoType::FileOrUrl(s) => {
                Clip::from_path(s.as_ref())?
            },
            IoType::Callback { filename, callback } => {
                // Install global custom IO
                let _io = CUSTOM_IO.get_or_init(move || {
                    let mut io = StreamIo::with_filesystem_fallback();
                    io.set_callback(move |path| {
                        match callback(path) {
                            Ok(IoType::Bytes(buffer)) => {
                                let size = buffer.len();
                                Some((Arc::new(std::sync::Mutex::new(std::io::Cursor::new(buffer))), Some(size as u64)))
                            },
                            Ok(IoType::ReadSeekStream { stream, size_hint }) => {
                                Some((Arc::new(std::sync::Mutex::new(stream)), size_hint))
                            },
                            Ok(IoType::ReadWriteSeekStream { stream, size_hint }) => {
                                Some((Arc::new(std::sync::Mutex::new(stream)), size_hint))
                            },
                            _ => None,
                        }
                    });
                    Mutex::new(CustomIO::install(Box::new(io)))
                });
                Clip::from_path(&filename)?
            },
            IoType::Bytes(buffer) => {
                if let Some(io) = CUSTOM_IO.get() {
                    let io = io.lock();
                    let stream_io = to_stream_io(&*io);
                    let size = buffer.len();
                    stream_io.insert(filename.unwrap_or("file.R3D").to_string(), std::io::Cursor::new(buffer), Some(size as u64));
                }
                Clip::from_path(filename.unwrap_or("file.R3D"))?
            },
            IoType::ReadSeekStream { stream, size_hint } => {
                if let Some(io) = CUSTOM_IO.get() {
                    let io = io.lock();
                    let stream_io = to_stream_io(&*io);
                    stream_io.insert(filename.unwrap_or("file.R3D").to_string(), stream, size_hint);
                }
                Clip::from_path(filename.unwrap_or("file.R3D"))?
            },
            IoType::ReadWriteSeekStream { stream, size_hint } => {
                if let Some(io) = CUSTOM_IO.get() {
                    let io = io.lock();
                    let stream_io = to_stream_io(&*io);
                    stream_io.insert(filename.unwrap_or("file.R3D").to_string(), stream, size_hint);
                }
                Clip::from_path(filename.unwrap_or("file.R3D"))?
            },
            IoType::FileList(map) => {
                let mut filenames = Vec::new();
                if let Some(io) = CUSTOM_IO.get() {
                    let io = io.lock();
                    let stream_io = to_stream_io(&*io);
                    for (name, item) in map {
                        let name_lower = name.to_ascii_lowercase();
                        if name_lower.contains(".r3d") || name_lower.contains(".nev") {
                            filenames.push(name.clone());
                        }
                        match item {
                            IoType::FileOrUrl(s) => {
                                filenames.push(s.to_string());
                            },
                            IoType::Bytes(buffer) => {
                                let size = buffer.len();
                                stream_io.insert(name.clone(), std::io::Cursor::new(buffer), Some(size as u64));
                            },
                            IoType::ReadSeekStream { stream, size_hint } => {
                                stream_io.insert(name.clone(), stream, size_hint);
                            },
                            IoType::ReadWriteSeekStream { stream, size_hint } => {
                                stream_io.insert(name.clone(), stream, size_hint);
                            },
                            _ => { return Err(VideoProcessingError::UnsupportedIO); }
                        }
                    }
                    filenames.sort();
                }
                let first_key = filenames.first().ok_or(VideoProcessingError::DecoderNotFound)?;
                Clip::from_path(first_key)?
            },
            _ => { return Err(VideoProcessingError::UnsupportedIO); }
        };

        let mut opts = R3dDecoderOptions::new()?;
        let _ = opts.set_memory_pool_size(4096);
        let _ = opts.set_gpu_memory_pool_size(4096);
        let _ = opts.set_gpu_concurrent_frame_count(3);
        let _ = opts.set_scratch_folder(""); // disable scratch folder
        let _ = opts.set_decompression_thread_count(0);
        let _ = opts.set_concurrent_image_count(0);

        // Select device options: prefer CUDA, fallback to OpenCL
        let mut device_set = false;
        if let Ok(list) = R3dDecoderOptions::cuda_device_list() {
            let mut iter = list.into_iter();
            let dev = if let Some(idx) = options.gpu_index { iter.nth(idx) } else { iter.next() };
            if let Some(dev) = dev {
                if opts.use_cuda_device(&dev).is_ok() {
                    log::debug!("R3D: Using CUDA device: {} (bus {})", dev.name(), dev.pci_bus_id());
                    device_set = true;
                }
            }
        }
        if !device_set {
            if let Ok(list) = R3dDecoderOptions::opencl_device_list() {
                let mut iter = list.into_iter();
                let dev = if let Some(idx) = options.gpu_index { iter.nth(idx) } else { iter.next() };
                if let Some(dev) = dev {
                    if opts.use_opencl_device(&dev).is_ok() {
                        log::debug!("R3D: Using OpenCL device: {} / {}", dev.platform_name(), dev.name());
                    }
                }
            }
        }

        let decoder = r3d_rs::R3dDecoder::new(&opts)?;

        // Build single video stream info
        let fps = clip.video_audio_framerate() as f64;
        let fps_rational = Rational((fps * 1000.0) as i32, 1000);
        let mut stream_state = Vec::new();
        stream_state.push(Stream {
            stream_type: StreamType::Video,
            index: 0,
            avg_frame_rate: fps_rational,
            rate:           fps_rational,
            time_base:      fps_rational.invert(),
            decode: true,
        });

        let frame_count = clip.video_frame_count() as u64;

        let mut mode = VideoDecodeMode::FullResPremium;
        let mut pixel_type = VideoPixelType::Bgra8bitInterleaved;

        if let Some(value) = select_custom_option(&options.custom_options, &["r3d.decode_resolution", "decode_resolution"]) {
            match parse_decode_mode(value) {
                Some(selected) => mode = selected,
                None => log::warn!("R3D: ignoring unknown decode_resolution '{value}'"),
            }
        }
        if let Some(value) = select_custom_option(&options.custom_options, &["r3d.output_format", "output_format"]) {
            match parse_pixel_type(value) {
                Some(selected) => pixel_type = selected,
                None => log::warn!("R3D: ignoring unknown output_format '{value}'"),
            }
        }

        let image_settings = clip.default_image_processing_settings();

        // Precompute size for buffer factory
        let size_bytes = clip.calculate_buffer_size(&mode, &pixel_type)?;
        let buffer_factory = R3dBufferFactory { size_bytes };
        let buffer_pool = Arc::new(BufferPool::new(8, buffer_factory));

        Ok(Self {
            clip,
            decoder,
            mode,
            pixel_type,
            image_settings,

            buffer_pool,
            frame_rate: fps,
            frame_count,
            current_frame: 0,
            open_options: options,
            stream_state,
        })
    }
}

// Helpers
fn mode_divisor(mode: &VideoDecodeMode) -> u32 {
    match mode {
        VideoDecodeMode::FullResPremium   => 1,
        VideoDecodeMode::HalfResPremium   => 2,
        VideoDecodeMode::HalfResGood      => 2,
        VideoDecodeMode::QuarterResGood   => 4,
        VideoDecodeMode::EightResGood     => 8,
        VideoDecodeMode::SixteenthResGood => 16,
    }
}
fn scaled_dims(src_w: u32, src_h: u32, mode: &VideoDecodeMode) -> (u32, u32) {
    let div = mode_divisor(mode);
    (src_w / div, src_h / div)
}
fn bytes_per_pixel(pt: VideoPixelType) -> usize {
    match pt {
        VideoPixelType::Bgra8bitInterleaved     => 4,
        VideoPixelType::Bgr8bitInterleaved      => 3,
        VideoPixelType::Rgb16bitInterleaved     => 6,
        VideoPixelType::RgbHalfFloatInterleaved => 6,
        VideoPixelType::RgbHalfFloatAcesInt     => 6,
        VideoPixelType::Rgb16bitPlanar          => 2,
        VideoPixelType::Dpx10bitMethodB         => 4,
    }
}

fn parse_decode_mode(value: &str) -> Option<VideoDecodeMode> {
    match value.to_ascii_lowercase().trim() {
        "full"      | "1"    => Some(VideoDecodeMode::FullResPremium),
        "half"               => Some(VideoDecodeMode::HalfResPremium),
        "half_good" | "1/2"  => Some(VideoDecodeMode::HalfResGood),
        "quarter"   | "1/4"  => Some(VideoDecodeMode::QuarterResGood),
        "eighth"    | "1/8"  => Some(VideoDecodeMode::EightResGood),
        "sixteenth" | "1/16" => Some(VideoDecodeMode::SixteenthResGood),
        _ => None,
    }
}

fn parse_pixel_type(value: &str) -> Option<VideoPixelType> {
    match value.to_ascii_lowercase().trim() {
        "bgra8"        => Some(VideoPixelType::Bgra8bitInterleaved),
        "bgr8"         => Some(VideoPixelType::Bgr8bitInterleaved),
        "rgb16"        => Some(VideoPixelType::Rgb16bitInterleaved),
        "rgb16_planar" => Some(VideoPixelType::Rgb16bitPlanar),
        "rgbf16"       => Some(VideoPixelType::RgbHalfFloatInterleaved),
        "rgbf16_aces"  => Some(VideoPixelType::RgbHalfFloatAcesInt),
        "dpx10"        => Some(VideoPixelType::Dpx10bitMethodB),
        _ => None,
    }
}

fn to_stream_io<'a>(io: &CustomIO<'a>) -> &'a StreamIo<'a> {
    let dyn_ioi: &dyn IoInterface = &**io.inner();
    // 1) widen to raw fat pointer
    let raw: *const dyn IoInterface = dyn_ioi;
    // 2) drop the vtable, keeping the thin data pointer
    let data: *const () = raw as *const ();
    // 3) reinterpret as *const MyIo and reborrow
    unsafe { &*(data as *const StreamIo) }
}
