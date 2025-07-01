set shell := ["powershell.exe", "-c"]

json-test:
    cargo build --features json-test
    ./target/debug/gameboy-advanced

bios-test:
    cargo build --features from-bios
    ./target/debug/gameboy-advanced "games/golden-sun.gba"

test TEST:
    cargo build
    ./target/debug/gameboy-advanced {{TEST}}

play GAME:
    cargo build
    ./target/debug/gameboy-advanced "games/{{GAME}}"