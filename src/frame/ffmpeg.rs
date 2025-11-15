// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

use super::*;
use ffmpeg_next::format::Pixel;


macro_rules! ffmpeg {
    ($func:stmt; $err:ident) => {
        let err = unsafe { $func };
        if err < 0 { return Err(crate::VideoProcessingError::$err(err)); }
    };
}

pub struct FfmpegVideoFrame {
    pub(crate) avframe: ffmpeg_next::frame::Video,
    pub(crate) swframe: Option<ffmpeg_next::frame::Video>
}

impl FfmpegVideoFrame {
    pub fn raw_frame(&self) -> &ffmpeg_next::frame::Video {
        &self.avframe
    }
    pub fn raw_sw_frame(&self) -> Option<&ffmpeg_next::frame::Video> {
        self.swframe.as_ref()
    }
}

impl VideoFrameInterface for FfmpegVideoFrame {
    fn width(&self)  -> u32 { self.avframe.width() }
    fn height(&self) -> u32 { self.avframe.height() }
    fn timestamp_us(&self) -> Option<i64> { self.avframe.timestamp() }

    fn format(&self) -> PixelFormat {
        let mut sw_format = self.avframe.format();
        unsafe {
            use ffmpeg_next::ffi::*;
            let hwctx = (*self.avframe.as_ptr()).hw_frames_ctx;
            if !hwctx.is_null() {
                let hwfc = (*hwctx).data as *const AVHWFramesContext;
                if !hwfc.is_null() {
                    sw_format = Pixel::from((*hwfc).sw_format);
                }
            }
        }

        match sw_format {
            Pixel::AYUV64LE    => PixelFormat::AYUV64LE,
            Pixel::NV12        => PixelFormat::NV12,
            Pixel::NV21        => PixelFormat::NV21,
            Pixel::NV16        => PixelFormat::NV16,
            Pixel::NV24        => PixelFormat::NV24,
            Pixel::NV42        => PixelFormat::NV42,
            Pixel::P010LE      => PixelFormat::P010LE,
            Pixel::P016LE      => PixelFormat::P016LE,
            Pixel::P210LE      => PixelFormat::P210LE,
            Pixel::P216LE      => PixelFormat::P216LE,
            Pixel::P410LE      => PixelFormat::P410LE,
            Pixel::P416LE      => PixelFormat::P416LE,
            //Pixel::RGB32       => PixelFormat::RGB32,
            //Pixel::RGB48BE     => PixelFormat::RGB48BE,
            Pixel::RGBA        => PixelFormat::RgbaU8,
            Pixel::BGRA        => PixelFormat::BgraU8,
            Pixel::RGBA64BE    => PixelFormat::RgbaU16, // TODO: check endianness
            Pixel::YUV420P     => PixelFormat::YUV420P,
            Pixel::YUVJ420P    => PixelFormat::YUV420P, // TODO: range
            Pixel::YUV420P10LE => PixelFormat::YUV420P10LE,
            Pixel::YUV420P12LE => PixelFormat::YUV420P12LE,
            Pixel::YUV420P14LE => PixelFormat::YUV420P14LE,
            Pixel::YUV420P16LE => PixelFormat::YUV420P16LE,
            Pixel::YUV422P     => PixelFormat::YUV422P,
            Pixel::YUVJ422P    => PixelFormat::YUV422P, // TODO: range
            Pixel::YUV422P10LE => PixelFormat::YUV422P10LE,
            Pixel::YUV422P12LE => PixelFormat::YUV422P12LE,
            Pixel::YUV422P14LE => PixelFormat::YUV422P14LE,
            Pixel::YUV422P16LE => PixelFormat::YUV422P16LE,
            Pixel::YUV444P     => PixelFormat::YUV444P,
            Pixel::YUVJ444P    => PixelFormat::YUV444P, // TODO: range
            Pixel::YUV444P10LE => PixelFormat::YUV444P10LE,
            Pixel::YUV444P12LE => PixelFormat::YUV444P12LE,
            Pixel::YUV444P14LE => PixelFormat::YUV444P14LE,
            Pixel::YUV444P16LE => PixelFormat::YUV444P16LE,
            Pixel::UYVY422     => PixelFormat::UYVY422,
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            Pixel::VIDEOTOOLBOX => {
                let pix_fmt = unsafe { mac_ffi::CVPixelBufferGetPixelFormatType((*self.avframe.as_ptr()).data[3] as mac_ffi::CVPixelBufferRef) };
                let pix_fmt_bytes = pix_fmt.to_be_bytes();
                match &pix_fmt_bytes {
                    b"BGRA" => PixelFormat::BgraU8,    // kCVPixelFormatType_32BGRA                        | 32 bit BGRA
                    b"xf20" => PixelFormat::P010LE,  // kCVPixelFormatType_420YpCbCr10BiPlanarFullRange  | 2 plane YCbCr10 4:2:0, each 10 bits in the MSBs of 16bits, full-range (Y range 0-1023)
                    b"x420" => PixelFormat::P010LE,  // kCVPixelFormatType_420YpCbCr10BiPlanarVideoRange | 2 plane YCbCr10 4:2:0, each 10 bits in the MSBs of 16bits, video-range (luma=[64,940] chroma=[64,960])
                    b"420f" => PixelFormat::NV12,    // kCVPixelFormatType_420YpCbCr8BiPlanarFullRange   | Bi-Planar Component Y'CbCr 8-bit 4:2:0, full-range (luma=[0,255] chroma=[1,255]).  baseAddr points to a big-endian CVPlanarPixelBufferInfo_YCbCrBiPlanar struct
                    b"420v" => PixelFormat::NV12,    // kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange  | Bi-Planar Component Y'CbCr 8-bit 4:2:0, video-range (luma=[16,235] chroma=[16,240]).  baseAddr points to a big-endian CVPlanarPixelBufferInfo_YCbCrBiPlanar struct
                    b"y420" => PixelFormat::YUV420P, // kCVPixelFormatType_420YpCbCr8Planar              | Planar Component Y'CbCr 8-bit 4:2:0.  baseAddr points to a big-endian CVPlanarPixelBufferInfo_YCbCrPlanar struct
                    b"f420" => PixelFormat::YUV420P, // kCVPixelFormatType_420YpCbCr8PlanarFullRange     | Planar Component Y'CbCr 8-bit 4:2:0, full range.  baseAddr points to a big-endian CVPlanarPixelBufferInfo_YCbCrPlanar struct
                    b"xf22" => PixelFormat::P210LE,  // kCVPixelFormatType_422YpCbCr10BiPlanarFullRange  | 2 plane YCbCr10 4:2:2, each 10 bits in the MSBs of 16bits, full-range (Y range 0-1023)
                    b"x422" => PixelFormat::P210LE,  // kCVPixelFormatType_422YpCbCr10BiPlanarVideoRange | 2 plane YCbCr10 4:2:2, each 10 bits in the MSBs of 16bits, video-range (luma=[64,940] chroma=[64,960])
                    b"sv22" => PixelFormat::P216LE,  // kCVPixelFormatType_422YpCbCr16BiPlanarVideoRange |
                    b"2vuy" => PixelFormat::UYVY422, // kCVPixelFormatType_422YpCbCr8                    | Component Y'CbCr 8-bit 4:2:2, ordered Cb Y'0 Cr Y'1
                    b"422f" => PixelFormat::NV16,    // kCVPixelFormatType_422YpCbCr8BiPlanarFullRange   |
                    b"422v" => PixelFormat::NV16,    // kCVPixelFormatType_422YpCbCr8BiPlanarVideoRange  |
                    b"y416" => PixelFormat::AYUV64LE,// kCVPixelFormatType_4444AYpCbCr16                 | Component Y'CbCrA 16-bit 4:4:4:4, ordered A Y' Cb Cr, full range alpha, video range Y'CbCr, 16-bit little-endian samples.
                    b"xf44" => PixelFormat::P410LE,  // kCVPixelFormatType_444YpCbCr10BiPlanarFullRange  | 2 plane YCbCr10 4:4:4, each 10 bits in the MSBs of 16bits, full-range (Y range 0-1023)
                    b"x444" => PixelFormat::P410LE,  // kCVPixelFormatType_444YpCbCr10BiPlanarVideoRange | 2 plane YCbCr10 4:4:4, each 10 bits in the MSBs of 16bits, video-range (luma=[64,940] chroma=[64,960])
                    b"sv44" => PixelFormat::P416LE,  // kCVPixelFormatType_444YpCbCr16BiPlanarVideoRange |
                    b"444f" => PixelFormat::NV24,    // kCVPixelFormatType_444YpCbCr8BiPlanarFullRange   |
                    b"444v" => PixelFormat::NV24,    // kCVPixelFormatType_444YpCbCr8BiPlanarVideoRange  |
                    _ => { log::error!("Unknown VT pixel format: {pix_fmt:08x}"); PixelFormat::Unknown }
                }
            },
            #[cfg(target_os = "windows")]
            Pixel::D3D11 => {
                use windows::{ Win32::Graphics::Direct3D11::*, Win32::Graphics::Dxgi::Common::*, core::Interface };

                let mut desc = D3D11_TEXTURE2D_DESC::default();
                unsafe {
                    let texture = (*self.avframe.as_ptr()).data[0] as *mut _;
                    // let index = (*self.avframe.as_ptr()).data[1] as i32;
                    ID3D11Texture2D::from_raw_borrowed(&texture).unwrap().GetDesc(&mut desc); // TODO: unwrap
                }
                match desc.Format {
                    DXGI_FORMAT_NV12               => PixelFormat::NV12,
                    DXGI_FORMAT_P010               => PixelFormat::P010LE,
                    DXGI_FORMAT_B8G8R8A8_UNORM     => PixelFormat::BgraU8,
                    // DXGI_FORMAT_R16G16B16A16_FLOAT => PixelFormat::RGBAF16,
                    DXGI_FORMAT_420_OPAQUE         => PixelFormat::YUV420P,
                    f => { log::error!("Unknown D3D11 pixel format: {f:?}"); PixelFormat::Unknown }
                }
            },
            #[cfg(target_os = "windows")]
            Pixel::DXVA2_VLD => {
                use windows::{ Win32::Graphics::Direct3D9::*, core::Interface };
                const NV12_F: u32 = u32::from_le_bytes(*b"NV12");
                const P010_F: u32 = u32::from_le_bytes(*b"P010");
                //const ARGB_F: u32 = D3DFMT_A8R8G8B8.0;
                //const BGRA_F: u32 = D3DFMT_B8G8R8A8.0;

                let mut desc = D3DSURFACE_DESC::default();
                unsafe {
                    let texture = (*self.avframe.as_ptr()).data[3] as *mut _;
                    if let Err(e) = IDirect3DSurface9::from_raw_borrowed(&texture).unwrap().GetDesc(&mut desc) { // TODO: unwrap
                        log::error!("Failed to get DXVA2 {}", e);
                    }
                }
                match desc.Format.0 {
                    NV12_F => PixelFormat::NV12,
                    P010_F => PixelFormat::P010LE,
                    //BGRA_F => PixelFormat::BgraU8,
                    f => { log::error!("Unknown DXVA pixel format: {f:08x}"); PixelFormat::Unknown }
                }
            },
            // #[cfg(target_os = "linux")]
            // Pixel::VAAPI => { let texture = unsafe { (*self.avframe.as_ptr()).data[3] as VASurfaceID }; },
            // #[cfg(target_os = "linux")]
            // Pixel::VDPAU => { let texture = unsafe { (*self.avframe.as_ptr()).data[3] as VdpVideoSurface }; },
            // #[cfg(any(target_os = "linux", target_os = "windows"))]
            // Pixel::QSV => { let texture = unsafe { (*self.avframe.as_ptr()).data[3] as *mut mfxFrameSurface1 }; },
            // #[cfg(any(target_os = "linux", target_os = "windows"))]
            // Pixel::CUDA => { let texture = unsafe {(*self.avframe.as_ptr()).data[0] as CUdeviceptr }; },
            // #[cfg(target_os = "android")]
            // Pixel::MEDIACODEC => { let texture = unsafe {(*self.avframe.as_ptr()).data[3] as *mut AVMediaCodecBuffer }; },*/
            f => {
                log::error!("Unknown pixel format: {f:?}");
                PixelFormat::Unknown
            }
        }
    }

    fn get_cpu_buffers(&mut self) -> Result<Vec<&mut [u8]>, crate::VideoProcessingError> {
        let input_frame =
            if unsafe { !(*self.avframe.as_mut_ptr()).hw_frames_ctx.is_null() } {
                if self.swframe.is_none() {
                    self.swframe = Some(ffmpeg_next::frame::Video::empty()); // TODO use buffer pool
                }
                let sw_frame = self.swframe.as_mut().unwrap();

                // let hw_formats = Some(unsafe { crate::support::ffmpeg_hw::get_transfer_formats_from_gpu(self.avframe.as_mut_ptr()) });
                // log::debug!("Hardware transfer formats from GPU: {:?}", hw_formats);
                // retrieve data from GPU to CPU
                ffmpeg!(ffmpeg_next::ffi::av_hwframe_transfer_data(sw_frame.as_mut_ptr(), self.avframe.as_mut_ptr(), 0); FromHWTransferError);
                ffmpeg!(ffmpeg_next::ffi::av_frame_copy_props(sw_frame.as_mut_ptr(), self.avframe.as_mut_ptr()); FromHWTransferError);
                sw_frame
            } else {
                &mut self.avframe
            };
        let mut ret = Vec::new();
        for index in 0..input_frame.planes() {
            // TODO: plane dimensions
            unsafe {
                ret.push(std::slice::from_raw_parts_mut((*input_frame.as_mut_ptr()).data[index], input_frame.stride(index) * input_frame.plane_height(index) as usize));
            }
        }
        Ok(ret)
    }

    fn get_gpu_texture(&mut self, plane: usize) -> Option<TextureDescription> {
        if unsafe { !(*self.avframe.as_mut_ptr()).hw_frames_ctx.is_null() } {
            match self.avframe.format() {
                /*#[cfg(any(target_os = "macos", target_os = "ios"))]
                Pixel::VIDEOTOOLBOX => {
                    Some (TextureDescription {
                        texture: HWTexture::VideoToolbox {
                            resource: ()
                        }
                    })
                },*/
                #[cfg(target_os = "windows")]
                Pixel::D3D11 => {
                    use windows::{ Win32::Graphics::Direct3D11::*, Win32::Graphics::Dxgi::Common::*, core::Interface };

                    let mut desc = D3D11_TEXTURE2D_DESC::default();
                    unsafe {
                        let texture = (*self.avframe.as_ptr()).data[0] as *mut _;
                        dbg!(texture);
                        // let index = (*self.avframe.as_ptr()).data[1] as i32;
                        ID3D11Texture2D::from_raw_borrowed(&texture)?.GetDesc(&mut desc);
                        dbg!(&desc);
                        None
                    }
                },
                #[cfg(target_os = "windows")]
                Pixel::DXVA2_VLD => {
                    use windows::{ Win32::Graphics::Direct3D9::*, core::Interface };
                    let mut desc = D3DSURFACE_DESC::default();
                    unsafe {
                        let texture = (*self.avframe.as_ptr()).data[3] as *mut _;
                        dbg!(texture);
                        //if let Err(e) = IDirect3DSurface9::from_raw_borrowed(&texture).GetDesc(&mut desc) {
                        //    log::error!("Failed to get DXVA2 {}", e);
                        //}
                        None
                    }
                },
                // #[cfg(target_os = "linux")]
                // Pixel::VAAPI => { let texture = unsafe { (*self.avframe.as_ptr()).data[3] as VASurfaceID }; },
                // #[cfg(target_os = "linux")]
                // Pixel::VDPAU => { let texture = unsafe { (*self.avframe.as_ptr()).data[3] as VdpVideoSurface }; },
                // #[cfg(any(target_os = "linux", target_os = "windows"))]
                // Pixel::QSV => { let texture = unsafe { (*self.avframe.as_ptr()).data[3] as *mut mfxFrameSurface1 }; },
                // #[cfg(any(target_os = "linux", target_os = "windows"))]
                // Pixel::CUDA => { let texture = unsafe {(*self.avframe.as_ptr()).data[0] as CUdeviceptr }; },
                // #[cfg(target_os = "android")]
                // Pixel::MEDIACODEC => { let texture = unsafe {(*self.avframe.as_ptr()).data[3] as *mut AVMediaCodecBuffer }; },
                f => {
                    log::error!("Unknown pixel format: {f:?}");
                    None
                }
            }
        } else {
            None
        }
    }

    fn color_range(&self) -> Option<ColorRange> {
        unsafe {
            use ffmpeg_next::ffi::AVColorRange::*;
            match (*self.avframe.as_ptr()).color_range {
                AVCOL_RANGE_UNSPECIFIED => None,
                AVCOL_RANGE_MPEG => Some(ColorRange::Limited),
                AVCOL_RANGE_JPEG => Some(ColorRange::Full),
                _ => None,
            }
        }
    }

    fn color_space(&self) -> Option<ColorSpace> {
        unsafe {
            use ffmpeg_next::ffi::AVColorSpace::*;
            match (*self.avframe.as_ptr()).colorspace {
                AVCOL_SPC_UNSPECIFIED => None,
                AVCOL_SPC_BT709 => Some(ColorSpace::Bt709),
                AVCOL_SPC_BT470BG | AVCOL_SPC_FCC | AVCOL_SPC_SMPTE170M | AVCOL_SPC_SMPTE240M => Some(ColorSpace::Bt601),
                AVCOL_SPC_BT2020_NCL | AVCOL_SPC_BT2020_CL | AVCOL_SPC_CHROMA_DERIVED_NCL | AVCOL_SPC_CHROMA_DERIVED_CL | AVCOL_SPC_ICTCP => Some(ColorSpace::Bt2020),
                _ => None,
            }
        }
    }
}

pub struct FfmpegAudioFrame {
    pub(crate) avframe: ffmpeg_next::frame::Audio
}

impl AudioFrameInterface for FfmpegAudioFrame {
    fn timestamp_us(&self) -> Option<i64> {
        self.avframe.timestamp()
    }
    fn buffer_size(&self) -> u32 {
        0
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod mac_ffi {
    #[derive(Debug, Copy, Clone)]
    pub enum __CVBuffer { }
    pub type CVBufferRef = *mut __CVBuffer;
    pub type CVImageBufferRef = CVBufferRef;
    pub type CVPixelBufferRef = CVImageBufferRef;

    #[link(name = "CoreVideo", kind = "framework")]
    unsafe extern "C" {
        pub fn CVPixelBufferGetPixelFormatType(pixelBuffer: CVPixelBufferRef) -> u32;
    }
}
