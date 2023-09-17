set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

BaseFile := os()

run *param:
    just --unstable -f _scripts/{{BaseFile}}.just run {{param}}

debug *param:
    just --unstable -f _scripts/{{BaseFile}}.just debug {{param}}

clippy *param:
    just --unstable -f _scripts/{{BaseFile}}.just clippy {{param}}

install-deps *param:
    just --unstable -f _scripts/{{BaseFile}}.just install-deps {{param}}
