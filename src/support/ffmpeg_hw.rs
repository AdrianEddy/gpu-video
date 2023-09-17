// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

use ffmpeg_next::{ ffi, format, codec, encoder };

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ffi::{ CStr, CString };
use std::ptr;
use parking_lot::Mutex;

type DeviceType = ffi::AVHWDeviceType;

#[derive(Debug)]
pub struct HWDevice {
    type_: DeviceType,
    device_ref: *mut ffi::AVBufferRef,
    device_name: Option<String>,

    pub hw_formats: Vec<format::Pixel>,
    pub sw_formats: Vec<format::Pixel>,
    pub min_size: (i32, i32),
    pub max_size: (i32, i32)
}
impl HWDevice {
    pub fn from_type(type_: DeviceType, device_name: Option<&str>) -> Result<Self, crate::VideoProcessingError> {
        log::debug!("HWDevice::from_type {type_:?}, device: {device_name:?}");
        unsafe {
            let dev = device_name.and_then(|x| if x.is_empty() { None } else { CString::new(x).ok() });

            let mut device_ref = ptr::null_mut();
            let err = ffi::av_hwdevice_ctx_create(&mut device_ref, type_, dev.as_ref().map_or(ptr::null(), |x| x.as_ptr()), ptr::null_mut(), 0);
            if err >= 0 && !device_ref.is_null() {
                Ok(Self {
                    type_,
                    device_name: device_name.map(|x| x.to_string()),
                    device_ref,
                    hw_formats: Vec::new(),
                    sw_formats: Vec::new(),
                    min_size: (0, 0),
                    max_size: (0, 0),
                })
            } else {
                log::error!("Failed to create specified HW device: {:?}", type_);
                Err(crate::VideoProcessingError::CannotCreateGPUDecoding)
            }
        }
    }

    pub fn add_ref(&self) -> *mut ffi::AVBufferRef {
        unsafe { ffi::av_buffer_ref(self.device_ref) }
    }
    pub fn as_mut_ptr(&self) -> *mut ffi::AVBufferRef { self.device_ref }
    pub fn device_type(&self) -> DeviceType { self.type_ }
    pub fn device_name(&self) -> Option<&str> { self.device_name.as_deref() }
    pub fn name(&self) -> String {
        unsafe {
            let name_ptr = ffi::av_hwdevice_get_type_name(self.type_);
            CStr::from_ptr(name_ptr).to_string_lossy().into()
        }
    }
}
impl Drop for HWDevice {
    fn drop(&mut self) {
        unsafe { ffi::av_buffer_unref(&mut self.device_ref); }
    }
}
unsafe impl Sync for HWDevice { }
unsafe impl Send for HWDevice { }

lazy_static::lazy_static! {
    static ref DEVICES: Mutex<HashMap<u64, HWDevice>> = Mutex::new(HashMap::new());
}

pub fn initialize_ctx(type_: ffi::AVHWDeviceType) {
    let mut devices = DEVICES.lock();
    if let Entry::Vacant(e) = devices.entry(type_ as u64) {
        ::log::debug!("create {:?}", type_);
        if let Ok(dev) = HWDevice::from_type(type_, None) {
            ::log::debug!("created ok {:?}", type_);
            e.insert(dev);
        }
    }
}

pub fn supported_gpu_backends() -> Vec<String> {
    let mut ret = Vec::new();
    let mut hw_type = ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE;
    for _ in 0..20 { // Better 20 than infinity
        unsafe {
            hw_type = ffi::av_hwdevice_iterate_types(hw_type);
            if hw_type == ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE {
                break;
            }
            // returns a pointer to static string, shouldn't be freed
            let name_ptr = ffi::av_hwdevice_get_type_name(hw_type);
            ret.push(CStr::from_ptr(name_ptr).to_string_lossy().into());
        }
    }
    ret
}

pub unsafe fn pix_formats_to_vec(formats: *const ffi::AVPixelFormat) -> Vec<format::Pixel> {
    let mut ret = Vec::new();
    for i in 0..100 {
        let p = *formats.offset(i);
        if p == ffi::AVPixelFormat::AV_PIX_FMT_NONE {
            break;
        }
        ret.push(p.into());
    }
    ret
}

pub fn init_device_for_decoding(index: usize, codec: *const ffi::AVCodec, decoder_ctx: &mut codec::context::Context, device: Option<&str>) -> Result<(usize, ffi::AVHWDeviceType, String, Option<ffi::AVPixelFormat>), crate::VideoProcessingError> {
    for i in index..20 {
        unsafe {
            let config = ffi::avcodec_get_hw_config(codec, i as i32);
            if config.is_null() {
                ::log::debug!("config null for {}", i);
                continue;
            }
            let type_ = (*config).device_type;
            if type_ == ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE {
                continue;
            }
            ::log::debug!("[dec] codec type {:?} {}", type_, i);
            let mut devices = DEVICES.lock();
            let mut device_hash = 0;
            if let Some(dev_name) = device {
                let mut hasher = crc32fast::Hasher::new();
                hasher.update(dev_name.as_bytes());
                device_hash = hasher.finalize() as u64;
            }
            if let Entry::Vacant(e) = devices.entry(type_ as u64 + device_hash) {
                if let Ok(dev) = HWDevice::from_type(type_, device) {
                    e.insert(dev);
                }
            }
            if let Some(dev) = devices.get(&(type_ as u64 + device_hash)) {
                (*decoder_ctx.as_mut_ptr()).hw_device_ctx = dev.add_ref();
                return Ok((i, type_, dev.name(), Some((*config).pix_fmt)));
            }
        }
    }
    Ok((0, ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE, String::new(), None))
}

pub fn find_working_encoder(encoders: &[(&'static str, bool)], device: Option<&str>) -> (&'static str, bool, Option<DeviceType>) {
    if encoders.is_empty() { return ("", false, None); } // TODO: should be Result<>

    let mut device_hash = 0;
    if let Some(dev_name) = device {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(dev_name.as_bytes());
        device_hash = hasher.finalize() as u64;
    }

    for x in encoders {
        if let Some(mut enc) = encoder::find_by_name(x.0) {
            if !x.1 { return (x.0, x.1, None); } // If not HW encoder

            for i in 0..20 {
                unsafe {
                    let type_ = if !x.0.contains("videotoolbox") {
                        let config = ffi::avcodec_get_hw_config(enc.as_mut_ptr(), i);
                        if config.is_null() {
                            println!("config is null {}", x.0);
                            break;
                        }
                        let type_ = (*config).device_type;
                        ::log::debug!("[enc] codec type {:?} {}, for: {}", type_, i, x.0);
                        let mut devices = DEVICES.lock();
                        if let Entry::Vacant(e) = devices.entry(type_ as u64 + device_hash) {
                            ::log::debug!("create {:?}", type_);
                            if let Ok(dev) = HWDevice::from_type(type_, device) {
                                ::log::debug!("created ok {:?}", type_);
                                e.insert(dev);
                            }
                        }
                        type_
                    } else {
                        ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VIDEOTOOLBOX
                    };
                    let mut devices = DEVICES.lock();
                    if let Some(dev) = devices.get_mut(&(type_ as u64 + device_hash)) {
                        let mut constraints = ffi::av_hwdevice_get_hwframe_constraints(dev.as_mut_ptr(), ptr::null());
                        if !constraints.is_null() {
                            dev.hw_formats = pix_formats_to_vec((*constraints).valid_hw_formats);
                            dev.sw_formats = pix_formats_to_vec((*constraints).valid_sw_formats);
                            dev.min_size = ((*constraints).min_width, (*constraints).min_height);
                            dev.max_size = ((*constraints).max_width, (*constraints).max_height);

                            log::debug!("HW formats: {:?}", &dev.hw_formats);
                            log::debug!("SW formats: {:?}", &dev.sw_formats);

                            ffi::av_hwframe_constraints_free(&mut constraints);
                        }
                        return (x.0, x.1, Some(dev.device_type()));
                    }
                }
            }
        } else {
            log::warn!("Codec not found: {:?}", x.0);
        }
    }
    let x = encoders.last().unwrap();
    (x.0, x.1, None)
}

pub unsafe fn get_transfer_formats_from_gpu(frame: *mut ffi::AVFrame) -> Vec<format::Pixel> {
    let mut formats = ptr::null_mut();
    if !frame.is_null() && !(*frame).hw_frames_ctx.is_null() {
        ffi::av_hwframe_transfer_get_formats((*frame).hw_frames_ctx, ffi::AVHWFrameTransferDirection::AV_HWFRAME_TRANSFER_DIRECTION_FROM, &mut formats, 0);
    }
    if formats.is_null() {
        Vec::new()
    } else {
        pix_formats_to_vec(formats)
    }
}
pub unsafe fn get_transfer_formats_to_gpu(frame: *mut ffi::AVFrame) -> Vec<format::Pixel> {
    let mut formats = ptr::null_mut();
    if !frame.is_null() && !(*frame).hw_frames_ctx.is_null() {
        ffi::av_hwframe_transfer_get_formats((*frame).hw_frames_ctx, ffi::AVHWFrameTransferDirection::AV_HWFRAME_TRANSFER_DIRECTION_TO, &mut formats, 0);
    }
    if formats.is_null() {
        Vec::new()
    } else {
        pix_formats_to_vec(formats)
    }
}

pub fn is_hardware_format(format: ffi::AVPixelFormat) -> bool {
    format == ffi::AVPixelFormat::AV_PIX_FMT_CUDA ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_DXVA2_VLD ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_VDPAU ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_D3D11 ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_D3D11VA_VLD ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_VIDEOTOOLBOX ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_MEDIACODEC ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_OPENCL ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_QSV ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_MMAL ||
    format == ffi::AVPixelFormat::AV_PIX_FMT_VAAPI
}

pub fn initialize_hwframes_context(encoder_ctx: *mut ffi::AVCodecContext, _frame_ctx: *mut ffi::AVFrame, type_: DeviceType, pixel_format: ffi::AVPixelFormat, size: (u32, u32), init_hwframes: bool, device_name: Option<&str>) -> Result<(), ()> {
    let mut devices = DEVICES.lock();
    let mut device_hash = 0;
    if let Some(dev_name) = device_name {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(dev_name.as_bytes());
        device_hash = hasher.finalize() as u64;
    }
    if let Some(dev) = devices.get_mut(&(type_ as u64 + device_hash)) {
        unsafe {
            if (*encoder_ctx).hw_device_ctx.is_null() {
                (*encoder_ctx).hw_device_ctx = dev.add_ref();
                log::debug!("Setting hw_device_ctx {:?}", (*encoder_ctx).hw_device_ctx);
            }

            if init_hwframes {
                if dev.sw_formats.is_empty() && !(*encoder_ctx).codec.is_null() {
                    dev.sw_formats = pix_formats_to_vec((*(*encoder_ctx).codec).pix_fmts);
                    log::debug!("Setting codec formats: {:?}", dev.sw_formats);
                }

                if !dev.hw_formats.is_empty() {
                    let target_format: ffi::AVPixelFormat = {
                        if !dev.sw_formats.contains(&pixel_format.into()) {
                            log::warn!("Encoder doesn't support the desired pixel format ({:?})\n", pixel_format);
                            log::debug!("dev.sw_formats: {:?}", &dev.sw_formats);
                            let formats = get_transfer_formats_to_gpu(_frame_ctx);
                            if formats.is_empty() {
                                log::warn!("No frame transfer formats. Desired format: {:?}", pixel_format);
                                ffi::AVPixelFormat::AV_PIX_FMT_NONE
                            } else if formats.contains(&pixel_format.into()) {
                                pixel_format
                            } else {
                                // Just pick the first format.
                                // TODO: this should maybe take into consideration if the frame is 8 bit or more
                                format::Pixel::into(*formats.first().unwrap())
                            }
                        } else {
                            pixel_format
                        }
                    };
                    log::debug!("target_format: {:?}", &target_format);

                    if target_format != ffi::AVPixelFormat::AV_PIX_FMT_NONE {
                        let hw_format = *dev.hw_formats.first().unwrap(); // Safe because we check !is_empty() above

                        if (*encoder_ctx).hw_frames_ctx.is_null() {
                            let mut hw_frames_ref = ffi::av_hwframe_ctx_alloc(dev.as_mut_ptr());
                            if hw_frames_ref.is_null() {
                                log::error!("Failed to create GPU frame context {:?}.", type_);
                                return Err(());
                            }
                            (*encoder_ctx).hw_frames_ctx = ffi::av_buffer_ref(hw_frames_ref);
                            ffi::av_buffer_unref(&mut hw_frames_ref);
                        } else {
                            log::debug!("hwframes already exists");
                        }
                        let mut frames_ctx_ref = (*encoder_ctx).hw_frames_ctx;

                        let frames_ctx = (*frames_ctx_ref).data as *mut ffi::AVHWFramesContext;
                        if (*frames_ctx).format    == ffi::AVPixelFormat::AV_PIX_FMT_NONE { (*frames_ctx).format    = hw_format.into(); }
                        if (*frames_ctx).sw_format == ffi::AVPixelFormat::AV_PIX_FMT_NONE { (*frames_ctx).sw_format = target_format; }
                        (*frames_ctx).width     = size.0 as i32;
                        (*frames_ctx).height    = size.1 as i32;
                        if type_ == ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_QSV || type_ == ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI {
                            (*frames_ctx).initial_pool_size = 20;
                        }

                        let err = ffi::av_hwframe_ctx_init(frames_ctx_ref);
                        if err < 0 {
                            log::error!("Failed to initialize frame context. Error code: {}", err);
                            ffi::av_buffer_unref(&mut frames_ctx_ref);
                            return Err(());
                        } else {
                            log::debug!("inited hwframe ctx");
                        }
                        log::debug!("frames_ctx.format: {:?}", &(*frames_ctx).format);
                        log::debug!("frames_ctx.sw_format: {:?}", &(*frames_ctx).sw_format);
                        (*encoder_ctx).pix_fmt = (*frames_ctx).format;
                    }
                }

            }
            return Ok(());
        }
    } else {
        log::warn!("DEVICES didn't have {:?}", type_);
    }
    Ok(())
}

pub fn find_best_matching_codec(codec: format::Pixel, supported: &[format::Pixel]) -> format::Pixel {
    if supported.is_empty() { return format::Pixel::None; }

    if supported.contains(&codec) { return codec; }

    let pairs = vec![
        (format::Pixel::P210LE, format::Pixel::YUV422P10LE),
        (format::Pixel::P010LE, format::Pixel::YUV420P10LE),
        (format::Pixel::NV12,   format::Pixel::YUV420P),
        (format::Pixel::NV21,   format::Pixel::YUV420P),
    ];
    for (a, b) in pairs {
        if codec == a && supported.contains(&b) { return b; }
        if codec == b && supported.contains(&a) { return a; }
    }

    log::warn!("No matching codec, we need {:?} and supported are: {:?}", codec, supported);

    *supported.first().unwrap()
}

// pub fn get_supported_pixel_formats(name: &str) -> Vec<ffi::AVPixelFormat> {
//     if let Some(mut codec) = encoder::find_by_name(name) {
//         unsafe {
//             pix_formats_to_vec((*codec.as_mut_ptr()).pix_fmts)
//         }
//     } else {
//         Vec::new()
//     }
// }
