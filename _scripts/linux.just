import 'common.just'

# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Dependencies ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

[no-cd]
install-deps:
    #!/bin/bash
    set -e

    sudo apt-get install -y p7zip-full clang libclang-dev git libva-dev libvdpau-dev libvdpau1 mesa-va-drivers ocl-icd-opencl-dev opencl-headers

    mkdir -p {{ExtDir}}

    if [[ ! -d "${LIBCLANG_PATH}" ]]; then
        sudo apt-get install -y libclang-13-dev
    fi

    pushd {{ExtDir}}

    if [ ! -d "{{FFmpeg}}" ]; then
        echo "Downloading ffmpeg"

        # Download and extract ffmpeg
        curl -L https://sourceforge.net/projects/avbuild/files/linux/{{FFmpeg}}.tar.xz/download -o ffmpeg.tar.xz
        7z x -aoa ffmpeg.tar.xz
        tar -xf ffmpeg.tar
        rm ffmpeg.tar ffmpeg.tar.xz
    fi

    popd

# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Development ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

[no-cd]
run *param:
    cargo run --release -- {{param}}

[no-cd]
debug *param:
    cargo run -- {{param}}
