// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

use super::*;
use crate::types::VideoProcessingError;
use crate::frame::ffmpeg::{ FfmpegAudioFrame, FfmpegVideoFrame };

use ffmpeg_next::{ codec, format, frame, media, Dictionary, rescale, rescale::Rescale };

pub enum OpenedDecoder {
    Video(ffmpeg_next::decoder::Video),
    Audio(ffmpeg_next::decoder::Audio)
}

struct StreamInfo {
    decoder: Option<OpenedDecoder>,
    info: Stream,
}

pub struct FfmpegDecoder {
    context: format::context::Input,
    current_packet: ffmpeg_next::Packet,

    packets_ended: bool,

    open_options: DecoderOptions,

    stream_state: Vec<StreamInfo>
}

impl DecoderInterface for FfmpegDecoder {
    fn streams(&mut self) -> Vec<&mut Stream> {
        self.stream_state.iter_mut().map(|x| &mut x.info).collect()
    }

    fn seek(&mut self, timestamp_us: i64) -> Result<bool, VideoProcessingError> {
        let position = timestamp_us.rescale((1, 1000000), rescale::TIME_BASE);
        if let Err(e) = self.context.seek(position, ..position) {
            log::error!("Failed to seek {:?}", e);
            return Err(VideoProcessingError::from(e));
        }
        Ok(true)
    }

    fn get_video_info(&self) -> Result<VideoInfo, VideoProcessingError> {
        let created_at = self.context.metadata().get("creation_time").and_then(|x| chrono::DateTime::parse_from_rfc3339(x).ok()).map(|x| x.timestamp_millis() as u64 / 1000);
        let metadata = self.context.metadata().iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>();
        if let Some(stream) = self.context.streams().best(media::Type::Video) {
            let codec = codec::context::Context::from_parameters(stream.parameters())?;
            if let Ok(video) = codec.decoder().video() {
                let mut bitrate = video.bit_rate();
                if bitrate == 0 { bitrate = self.context.bit_rate() as usize; }

                let mut frames = stream.frames() as usize;
                if frames == 0 { frames = (stream.duration() as f64 * f64::from(stream.time_base()) * f64::from(stream.rate())) as usize; }

                let rotation = {
                    let mut theta = 0.0;
                    if let Some(rotate_tag) = stream.metadata().get("rotate") {
                        if let Ok(num) = rotate_tag.parse::<f64>() {
                            theta = num;
                        }
                    }
                    if theta == 0.0 {
                        for side_data in stream.side_data() {
                            if side_data.kind() == codec::packet::side_data::Type::DisplayMatrix {
                                let display_matrix = side_data.data();
                                if display_matrix.len() == 9*4 {
                                    theta = -unsafe { ffmpeg_next::ffi::av_display_rotation_get(display_matrix.as_ptr() as *const i32) };
                                }
                            }
                        }
                    }

                    theta -= 360.0 * (theta / 360.0 + 0.9 / 360.0).floor();
                    theta as i32
                };

                return Ok(VideoInfo {
                    duration_ms: stream.duration() as f64 * f64::from(stream.time_base()) * 1000.0,
                    frame_count: frames,
                    fps: f64::from(stream.rate()), // or avg_frame_rate?
                    width: video.width(),
                    height: video.height(),
                    bitrate: bitrate as f64 / 1024.0 / 1024.0,
                    rotation,
                    created_at,
                    metadata
                });
            }
        }
        Err(ffmpeg_next::Error::StreamNotFound.into())
    }

    fn next_frame(&mut self) -> Result<Option<Frame>, VideoProcessingError> {
        let fetch_new_packet = unsafe { self.current_packet.is_empty() };
        if fetch_new_packet && !self.packets_ended {
            loop {
                match self.current_packet.read(&mut self.context) {
                    Ok(..) => { break; },
                    Err(ffmpeg_next::Error::Eof) => {
                        self.packets_ended = true;
                        for state in &mut self.stream_state {
                            match &mut state.decoder {
                                Some(OpenedDecoder::Video(decoder)) => decoder.send_eof()?,
                                Some(OpenedDecoder::Audio(decoder)) => decoder.send_eof()?,
                                _ => { }
                            }
                        }
                        break;
                    },
                    Err(e) => { println!("other err {e:?}"); },
                }
            }
        }

        let stream = unsafe { ffmpeg_next::Stream::wrap(&self.context, self.current_packet.stream()) };

        let state = &mut self.stream_state[stream.index()];

        if state.info.decode && state.decoder.is_none() {
            let mut ctx = codec::context::Context::from_parameters(stream.parameters())?;
            state.decoder = match stream.parameters().medium() {
                media::Type::Video => {
                    ctx.set_threading(ffmpeg_next::threading::Config { kind: ffmpeg_next::threading::Type::Frame, count: 3 });

                    // let mut hw_backend = String::new();
                    let mut codec = ffmpeg_next::decoder::find(ctx.id()).ok_or(VideoProcessingError::DecoderNotFound)?;

                    if let Some(gpu_index) = self.open_options.gpu_index {
                        let hwaccel_device = self.open_options.custom_options.get("hwaccel_device").cloned();

                        let hw = crate::support::ffmpeg_hw::init_device_for_decoding(gpu_index, unsafe { codec.as_ptr() }, &mut ctx, hwaccel_device.as_deref())?;
                        // log::debug!("Selected HW backend {:?} ({}) with format {:?}", hw.1, hw.2, hw.3);
                        // hw_backend = hw.2;
                    }

                    Some(OpenedDecoder::Video(ctx.decoder().open_as(codec).and_then(|o| o.video())?))
                },
                media::Type::Audio => Some(OpenedDecoder::Audio(ctx.decoder().audio()?)),
                _ => None
            };
        }

        let mut decoder = match state.decoder.as_mut() {
            Some(OpenedDecoder::Video(decoder)) => Some(&mut decoder.0),
            Some(OpenedDecoder::Audio(decoder)) => Some(&mut decoder.0),
            _ => None
        };
        if let Some(decoder) = decoder {
            if fetch_new_packet && !self.packets_ended {
                self.current_packet.rescale_ts(stream.time_base(), (1, 1000000)); // rescale to microseconds

                if let Err(e) = decoder.send_packet(&self.current_packet) {
                    log::error!("Decode error: {:?}", e);
                    return Err(e.into());
                }
            }
            let mut frame = unsafe { ffmpeg_next::Frame::empty() };
            if let Err(e) = decoder.receive_frame(&mut frame) {
                self.current_packet = ffmpeg_next::Packet::empty();
                if self.packets_ended { return Ok(None); }
                return self.next_frame();
            }

            match stream.parameters().medium() {
                media::Type::Video => {
                    Ok(Some(Frame::Video(FfmpegVideoFrame { avframe: frame::Video::from(frame), swframe: None }.into())))
                },
                media::Type::Audio => {
                    Ok(Some(Frame::Audio(FfmpegAudioFrame { avframe: frame::Audio::from(frame) }.into())))
                },
                // media::Type::Subtitle => {
                //     Some(Frame::Subtitle(FfmpegSubtitleFrame {  }.into()))
                // },
                _ => {
                    self.current_packet = ffmpeg_next::Packet::empty();
                    Ok(Some(Frame::Other))
                }
            }
        } else {
            self.current_packet = ffmpeg_next::Packet::empty();
            if self.packets_ended { return Ok(None); }
            Ok(Some(Frame::Other))
        }
    }
}

impl FfmpegDecoder {
    pub fn new<'a>(input: IoType<'a>, filename: Option<&str>, options: DecoderOptions) -> Result<Self, VideoProcessingError> {
        use format::{ context::StreamIo, input_from_stream };
        use std::io::Cursor;

        ffmpeg_next::init()?;

        let mut options_avdict = Dictionary::new();
        for (k, v) in &options.custom_options { options_avdict.set(&k, &v); }

        let mut input_context = match input {
            IoType::FileOrUrl(mut s) => {
                if s.starts_with("fd:") {
                    options_avdict.set("fd", &s[3..]);
                    s = std::borrow::Cow::Borrowed("fd:");
                }
                format::input_with_dictionary(s.as_ref(), options_avdict)?
            },
            IoType::Bytes(m)                           => { input_from_stream(StreamIo::from_read_seek(Cursor::new(m))?, filename, Some(options_avdict))? },
            IoType::ReadStream          { stream, .. } => { input_from_stream(StreamIo::from_read(stream)?,              filename, Some(options_avdict))? },
            IoType::ReadSeekStream      { stream, .. } => { input_from_stream(StreamIo::from_read_seek(stream)?,         filename, Some(options_avdict))? },
            IoType::WriteStream         { stream, .. } => { input_from_stream(StreamIo::from_write(stream)?,             filename, Some(options_avdict))? },
            IoType::WriteSeekStream     { stream, .. } => { input_from_stream(StreamIo::from_write_seek(stream)?,        filename, Some(options_avdict))? },
            IoType::ReadWriteSeekStream { stream, .. } => { input_from_stream(StreamIo::from_read_write_seek(stream)?,   filename, Some(options_avdict))? },
            _ => {
                log::error!("Unknown input");
                return Err(VideoProcessingError::DecoderNotFound);
            }
        };

        // format::context::input::dump(&input_context, 0, Some(path));

        let mut stream_state = Vec::new();

        for (i, stream) in input_context.streams().enumerate() {
            let medium = stream.parameters().medium();
            let stream_type = match medium {
                media::Type::Video => StreamType::Video,
                media::Type::Audio => StreamType::Audio,
                media::Type::Subtitle => StreamType::Subtitle,
                _ => StreamType::Other,
            };

            let avg_fps = stream.avg_frame_rate();
            let rate = stream.rate();
            let time_base = stream.time_base();

            stream_state.push(StreamInfo {
                decoder: None,
                info: Stream {
                    stream_type,
                    index: i,
                    avg_frame_rate: Rational(avg_fps.0, avg_fps.1),
                    rate:           Rational(rate.0, rate.1),
                    time_base:      Rational(time_base.0, time_base.1),

                    decode: true,
                }
            });
        }

        Ok(Self {
            context: input_context,
            current_packet: ffmpeg_next::Packet::empty(),

            packets_ended: false,
            open_options: options,

            stream_state
        })
    }
}
