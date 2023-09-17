!include common.just

# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Dependencies ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

[no-cd]
install-deps:
    #!powershell
    $ProgressPreference = 'SilentlyContinue'
    $ErrorActionPreference = 'Stop'

    mkdir "{{ExtDir}}" -ErrorAction SilentlyContinue
    cd {{ExtDir}}

    # ――――――――――――――――――――――――――――――――――――――― FFmpeg ――――――――――――――――――――――――――――――――――――――――
    if (-not (Test-Path -Path "{{FFmpeg}}")) {
        echo "Downloading {{FFmpeg}}"
        wget "https://master.dl.sourceforge.net/project/avbuild/windows-desktop/{{FFmpeg}}.7z?viasf=1" -outfile "{{FFmpeg}}.7z"
        7z x -y {{FFmpeg}}.7z
        del {{FFmpeg}}.7z
    }
    # ――――――――――――――――――――――――――――――――――――――― FFmpeg ――――――――――――――――――――――――――――――――――――――――

    # ――――――――――――――――――――――――――――――――――――――― LLVM ――――――――――――――――――――――――――――――――――――――――
    if (-not (Test-Path -Path "{{LLVMPath}}\libclang.dll")) {
        echo "Downloading LLVM"
        wget "https://github.com/llvm/llvm-project/releases/download/llvmorg-16.0.0/LLVM-16.0.0-win64.exe" -outfile "llvm-win64.exe"
        7z x -y llvm-win64.exe -ollvm
        del "llvm-win64.exe"
    }
    # ――――――――――――――――――――――――――――――――――――――― LLVM ――――――――――――――――――――――――――――――――――――――――

# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Development ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
# ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

[no-cd]
run *param:
    cargo run --release -- {{param}}

[no-cd]
clippy:
    cargo clippy

[no-cd]
debug *param:
    cargo run -- {{param}}
