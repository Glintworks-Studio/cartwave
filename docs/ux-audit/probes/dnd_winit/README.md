# dnd_winit

Raw winit 0.30.13 window that logs every WindowEvent (and polls GetCursorPos while a file hovers). Build: `CARGO_TARGET_DIR=../target ~/.cargo/bin/cargo build --release` here; run `../target/release/dnd_winit.exe` from this folder (log: `dnd_winit.log`), then `pwsh -File ../../tools/oledrag.ps1 -Title DndWinitProbe -File <any file> -Mode drop|leave|escape`.
