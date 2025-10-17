// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright © 2023 Adrian <adrian.eddy at gmail>

fn main() {
    if cfg!(not(feature = "link-libs")) {
        return;
    }

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "ios" {
        let frameworks = [
            "AudioToolbox", "AVFoundation", "CoreAudio", "CoreFoundation",
            "CoreGraphics", "CoreMedia", "CoreServices", "CoreText",
            "CoreVideo", "Foundation", "ImageIO", "IOKit", "CFNetwork",
            "OpenGLES", "QuartzCore", "Security", "SystemConfiguration",
            "UIKit", "UniformTypeIdentifiers", "VideoToolbox", "Photos"
        ];

        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=bz2");
        println!("cargo:rustc-link-lib=xml2");
        for x in frameworks {
            println!("cargo:rustc-link-lib=framework={x}");
        }
    } else if target_os == "macos" {
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=bz2");
        println!("cargo:rustc-link-lib=xml2");
        println!("cargo:rustc-link-lib=framework=AudioToolbox");
        println!("cargo:rustc-link-lib=framework=VideoToolbox");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=CoreAudio");
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=CoreServices");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=OpenGL");
        println!("cargo:rustc-link-lib=framework=CFNetwork");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-arg=-lc++");
    }
    if cfg!(feature = "ffmpeg") {
        match target_os.as_str() {
            "android" => {
                println!("cargo:rustc-link-search={}/lib/arm64-v8a", std::env::var("FFMPEG_DIR").unwrap());
                println!("cargo:rustc-link-search={}/lib", std::env::var("FFMPEG_DIR").unwrap());
            },
            "macos" | "ios" => {
                println!("cargo:rustc-link-search={}/lib", std::env::var("FFMPEG_DIR").unwrap());
                println!("cargo:rustc-link-lib=static:+whole-archive,-bundle=x264");
                println!("cargo:rustc-link-lib=static:+whole-archive,-bundle=x265");
            },
            "linux" => {
                println!("cargo:rustc-link-search={}/lib/amd64", std::env::var("FFMPEG_DIR").unwrap());
                println!("cargo:rustc-link-search={}/lib", std::env::var("FFMPEG_DIR").unwrap());
                println!("cargo:rustc-link-lib=static:+whole-archive=z");
            },
            "windows" => {
                println!("cargo:rustc-link-search={}\\lib\\x64", std::env::var("FFMPEG_DIR").unwrap());
                println!("cargo:rustc-link-search={}\\lib", std::env::var("FFMPEG_DIR").unwrap());
            }
            tos => panic!("unknown target os {:?}!", tos)
        }
    }
}
