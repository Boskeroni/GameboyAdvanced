set shell := ["powershell.exe", "-c"]

json-test:
    cargo build -q --features json-test
    ./target/debug/gameboy-advanced

bios-test:
    cargo build -q --features from-bios
    ./target/debug/gameboy-advanced "games/golden-sun.gba"

test TEST:
    cargo build -q
    ./target/debug/gameboy-advanced "{{TEST}}.gba"

play GAME:
    cargo build -q
    ./target/debug/gameboy-advanced "games/{{GAME}}.gba"