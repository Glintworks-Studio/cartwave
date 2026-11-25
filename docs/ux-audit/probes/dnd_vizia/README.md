# dnd_vizia

OS file drop on a vizia window: what the fork delivers (stock) and a replacement OLE IDropTarget that reports the hover position (`--own-droptarget`). Build: `CARGO_TARGET_DIR=../target ~/.cargo/bin/cargo build --release` here; run `../target/release/dnd_vizia.exe [--own-droptarget]` from this folder (log: `dnd_vizia.log`), then `pwsh -File ../../tools/oledrag.ps1 -Title DndViziaProbe -File <any file> -Mode drop|leave|escape`.
