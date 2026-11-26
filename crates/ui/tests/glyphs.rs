//! Text the app draws must use characters its bundled Inter has (BATCH4A_PROMPT.md §3.3).
//! Inter has no "▸" (U+25B8) and no "🔒" (U+1F512): checked against the fonts' `cmap` with
//! `docs/batch4a/tools/cmap_check.py` (results in `docs/batch4a/cmap.txt`). Menu paths in
//! messages use ">" as the clip panel does; the lock is drawn, not typed.

const MISSING: [char; 2] = ['\u{25B8}', '\u{1F512}'];

fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn no_string_literal_uses_a_character_inter_lacks() {
    let mut files = Vec::new();
    sources(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    let mut bad = Vec::new();
    for f in files {
        for (i, line) in std::fs::read_to_string(&f).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // Only inside string literals (a quote before the character).
            for c in MISSING {
                if let Some(at) = line.find(c) {
                    if line[..at].contains('"') {
                        bad.push(format!("{}:{}: {}", f.display(), i + 1, line.trim()));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "characters Inter can't draw:\n{}", bad.join("\n"));
}
