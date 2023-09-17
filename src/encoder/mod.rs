// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

use std::collections::HashMap;

pub struct Encoder {

}

pub enum EncoderCodec {
    H264, H265, ProRes, DNxHR, PNG, EXR
}
pub enum Bitrate {
    Constant(f64), // in Mbps
    Variable((f64, f64)), // min, max in Mbps
    QScale(f64)
}

pub struct EncoderParams {
    width: u32,
    height: u32,
    format: crate::types::PixelFormat,
    bitrate: Bitrate,
    codec: EncoderCodec,
    use_gpu: bool,
    frame_rate: f32,
    time_base: Option<(u32, u32)>,
    custom_options: HashMap<String, String>,

    color_range_full: bool,
    // color_space: Option<ColorSpace>,
    // color_trc: Option<ColorTrc>,
    // color_primaries: Option<ColorPrimaries>,
    // aspect_ratio: Option<(u32, u32)>,
}
