# menu_vizia

muda 0.22 native File/Edit menu bar on a vizia window (ChipDAW fork). Build: `CARGO_TARGET_DIR=../target ~/.cargo/bin/cargo build --release` in this folder; run `../target/release/menu_vizia.exe [--accel none|hook|hook-swallow]` from this folder (log: `menu_vizia.log`); drive it with `../../tools/uxdrive.ps1 -Title MenuProbe -Do "activate; keys ctrl+s; keys alt+f; wait 1500; keys esc esc"`.
