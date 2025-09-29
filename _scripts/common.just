set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
PathSep := if os() == "windows" { ";" } else { ":" }

ProjectDir := if path_exists(justfile_directory() / "Cargo.toml") == "true" { justfile_directory()
       } else if path_exists(justfile_directory() / ".." / "Cargo.toml") == "true" { justfile_directory() / "../"
       } else { justfile_directory() }
ExtDir := ProjectDir / "ext"

# ――――――――――――――――――――――――――――――――――――――― FFmpeg ――――――――――――――――――――――――――――――――――――――――
FFmpeg := if os() == "windows" { "ffmpeg-8.0-windows-desktop-vs2022-gpl-lite"
   } else if os() + arch() == "macosaarch64" { "ffmpeg-arm64"
   } else if os() + arch() == "macosx86_64"  { "ffmpeg-x64_64"
   } else                      { "ffmpeg-8.0-linux-clang-gpl-lite" }
export FFMPEG_DIR := ProjectDir / "ext" / FFmpeg
# ――――――――――――――――――――――――――――――――――――――― FFmpeg ――――――――――――――――――――――――――――――――――――――――

# ――――――――――――――――――――――――――――――――――――――― MacOS ―――――――――――――――――――――――――――――――――――――――――
export DYLD_FALLBACK_LIBRARY_PATH := if os() == "macos" { `xcode-select --print-path` + "/Toolchains/XcodeDefault.xctoolchain/usr/lib/" } else { "" }
export MACOSX_DEPLOYMENT_TARGET := "10.15"
# ――――――――――――――――――――――――――――――――――――――― MacOS ―――――――――――――――――――――――――――――――――――――――――

# ――――――――――――――――――――――――――――――――――――――― Clang ―――――――――――――――――――――――――――――――――――――――――
export LIBCLANG_PATH := if os() == "macos" { DYLD_FALLBACK_LIBRARY_PATH } else { if path_exists(ExtDir / "llvm/bin") == "true" { ExtDir / "llvm/bin" } else { env_var_or_default("LIBCLANG_PATH", if path_exists("/usr/lib/llvm-13/lib/") == "true" { "/usr/lib/llvm-13/lib/" } else { "" }) } }
LLVMPath := LIBCLANG_PATH
# ――――――――――――――――――――――――――――――――――――――― Clang ―――――――――――――――――――――――――――――――――――――――――

export LD_LIBRARY_PATH := FFMPEG_DIR + "/lib:" + FFMPEG_DIR + "/lib/amd64"

export PATH := FFMPEG_DIR + "/bin" + PathSep + FFMPEG_DIR + "/bin/amd64" + PathSep + LLVMPath + PathSep + env_var('PATH')
