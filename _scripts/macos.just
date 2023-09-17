!include common.just

# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Dependencies ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

[no-cd]
install-deps:
    #!/bin/bash
    set -e

    brew install p7zip
    xcode-select --install || true

    mkdir -p {{ExtDir}}
    cd {{ExtDir}}

    if [ ! -d "{{FFmpeg}}" ]; then
        echo "Downloading ffmpeg"

        # Download and extract ffmpeg
        curl -L https://sourceforge.net/projects/avbuild/files/macOS/ffmpeg-master-macOS-gpl-lite.tar.xz/download -o ffmpeg.tar.xz
        7z x -aoa ffmpeg.tar.xz
        tar -xf ffmpeg.tar
        rm ffmpeg.tar ffmpeg.tar.xz
        mkdir -p ffmpeg-x86_64/lib
        mkdir -p ffmpeg-arm64/lib
        cd ffmpeg-master-macOS-gpl-lite
        lipo lib/libavcodec.a    -thin x86_64 -output ../ffmpeg-x86_64/lib/libavcodec.a
        lipo lib/libavformat.a   -thin x86_64 -output ../ffmpeg-x86_64/lib/libavformat.a
        lipo lib/libavdevice.a   -thin x86_64 -output ../ffmpeg-x86_64/lib/libavdevice.a
        lipo lib/libavfilter.a   -thin x86_64 -output ../ffmpeg-x86_64/lib/libavfilter.a
        lipo lib/libavutil.a     -thin x86_64 -output ../ffmpeg-x86_64/lib/libavutil.a
        lipo lib/libswresample.a -thin x86_64 -output ../ffmpeg-x86_64/lib/libswresample.a
        lipo lib/libswscale.a    -thin x86_64 -output ../ffmpeg-x86_64/lib/libswscale.a
        lipo lib/libx264.a       -thin x86_64 -output ../ffmpeg-x86_64/lib/libx264.a
        lipo lib/libx265.a       -thin x86_64 -output ../ffmpeg-x86_64/lib/libx265.a

        lipo lib/libavcodec.a    -thin arm64 -output ../ffmpeg-arm64/lib/libavcodec.a
        lipo lib/libavformat.a   -thin arm64 -output ../ffmpeg-arm64/lib/libavformat.a
        lipo lib/libavdevice.a   -thin arm64 -output ../ffmpeg-arm64/lib/libavdevice.a
        lipo lib/libavfilter.a   -thin arm64 -output ../ffmpeg-arm64/lib/libavfilter.a
        lipo lib/libavutil.a     -thin arm64 -output ../ffmpeg-arm64/lib/libavutil.a
        lipo lib/libswresample.a -thin arm64 -output ../ffmpeg-arm64/lib/libswresample.a
        lipo lib/libswscale.a    -thin arm64 -output ../ffmpeg-arm64/lib/libswscale.a
        lipo lib/libx264.a       -thin arm64 -output ../ffmpeg-arm64/lib/libx264.a
        lipo lib/libx265.a       -thin arm64 -output ../ffmpeg-arm64/lib/libx265.a
        cp -R include ../ffmpeg-x86_64/include
        cp -R include ../ffmpeg-arm64/include
    fi

# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Development ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

[no-cd]
run *param:
    cargo run --release -- {{param}}

[no-cd]
debug *param:
    cargo run -- {{param}}
