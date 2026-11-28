//! The preset bank's listening pack, level report and provenance table
//! (PRESETS_PROMPT.md §7.3–§7.5).
//!
//! usage: cargo run --release -p ui --example bank_pack -- [--write] [--iterate] [--measure "A,B"]
//! (`--iterate`: also rewrite `presets/levels.json`, a level-matching pass)
//!
//! Every factory preset (`ui::bank::all()`) plays its role's phrase
//! (`bank::PHRASES`, 150 BPM) on one track: a studio track of its voice, or,
//! for an authentic preset, a track locked to the first channel of its voice
//! under its hardware profile (the console's mixer and filter). Each render
//! is measured (EBU R128 integrated loudness, true peak, sample peak) and
//! checked for clipping (no sample at or past full scale).
//!
//! Level matching: each category (a Sounds or Drums role folder; Hardware by
//! profile and channel) has a target, fixed from the first pass at the 35th
//! percentile of its new presets; `presets/levels.json` gets the gain that brings each new preset
//! to it, which `presets/tools/design_bank.py --levels` folds into the
//! preset (the classic presets are reported, not changed: RUNLOG PB-D6).
//!
//! Authentic presets are also rendered on a studio voice: the report says
//! which render bit-identically (§7.3).
//!
//! Without `--write` it only measures and prints; with it, it writes
//! `docs/audio/presets/` (one WAV per preset, `index.html`, `levels.md`),
//! `presets/levels.json` and `presets/PROVENANCE.md`.

use std::collections::BTreeMap;
use std::sync::Arc;

use ui::bank::{self, Preset};
use ui::file::Project;
use ui::state::{Clip, SoundChip, TICKS_PER_BAR};
use ui::theme::tokens::Rgba;

/// A project playing preset `p`'s phrase: on a studio track, or (`locked`)
/// on a track locked to the first channel of its voice under its profile.
pub fn project_for(p: &Preset, locked: bool) -> Project {
    let mut pr = Project::new_empty();
    pr.arrangement.lanes.clear();
    pr.arrangement.instruments.lanes.clear();
    let lane = pr.arrangement.insert_track(0, p.voice, Rgba::hex(0x4FA8E0));
    // An authentic preset plays under its profile (its output filter), on a
    // locked track or, for the comparison, on a studio track.
    if let Some(prof) = p.profile {
        pr.chip = prof;
        pr.filter = ui::tracks::default_filter(prof);
    }
    if locked {
        let chan = ui::tracks::channels_of(p.voice)[0];
        pr.arrangement.lock_track(lane, chan);
    }
    bank::apply(&mut pr.arrangement, lane, p);
    let ph = bank::phrase_for(&p.phrase);
    let notes = bank::phrase_notes(ph, p.octave);
    let len = ph.bars * TICKS_PER_BAR;
    pr.arrangement.clips.push(Arc::new(Clip::new(1, lane, 0, len, ph.name.into(), notes)));
    pr.loop_end_ticks = len;
    pr
}

/// File-safe slug: lowercase letters, digits, dots and dashes.
fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() || c == '.' {
            out.push(c);
        } else if c == '&' {
            out.push_str("and");
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn rel_path(p: &Preset) -> String {
    let dir: Vec<String> = p.path.iter().map(|s| slug(s)).collect();
    format!("{}/{}.wav", dir.join("/"), slug(&p.name))
}

/// The level-matching category: a Sounds / Drums role, or a Hardware
/// profile's channel.
fn category(p: &Preset) -> String {
    let n = if p.path[0] == "Hardware" { 3 } else { 2 };
    p.path[..n.min(p.path.len())].join(" / ")
}

/// Dated change lines for presets changed after they shipped.
fn change_line(name: &str) -> &'static str {
    match name {
        "Music Box" | "Pizzicato" | "Menu Blip" => "; **changed 2026-11-28** (Batch 5 A2): level only, raised to the voice's maximum (channel Volume 15, the volume macro scaled up to a loudest step of 15, its shape kept); duty, pitch, arpeggio and vibrato unchanged",
        _ => "",
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

struct Row {
    i: usize,
    lufs: f64,
    tp: f64,
    peak: f64,
    clips: bool,
    secs: f64,
    /// Authentic: the studio render's hash equals the locked one.
    same_on_studio: Option<bool>,
    /// Authentic: the difference's level against the locked render (dB) and
    /// the loudness difference (LU).
    residual: Option<(f64, f64)>,
}

fn main() {
    let write = std::env::args().any(|a| a == "--write");
    // `--measure "A,B"`: only print these presets' loudness, true peak and
    // sample peak (Batch 5 A2's quick check), then stop.
    let args: Vec<String> = std::env::args().collect();
    if let Some(k) = args.iter().position(|a| a == "--measure") {
        let names: Vec<&str> = args.get(k + 1).map_or("", |s| s.as_str()).split(',').collect();
        for p in bank::all().iter().filter(|p| names.iter().any(|n| n.eq_ignore_ascii_case(&p.name))) {
            let (x, ch) = ui::export::render_channels(&project_for(p, p.authentic()));
            let (lufs, tp, peak) = ui::game_export::measure_ch(&x, 48_000, ch).unwrap_or((f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY));
            println!("{}	{lufs:.2} LUFS	tp {tp:.2}	peak {peak:.2}	clips {}", p.name, x.iter().any(|v| v.abs() >= 1.0));
        }
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = root.join("docs/audio/presets");
    let all = bank::all();
    let mut rows = Vec::new();
    for (i, p) in all.iter().enumerate() {
        let pr = project_for(p, p.authentic());
        let (x, ch) = ui::export::render_channels(&pr);
        let (lufs, tp, peak) = ui::game_export::measure_ch(&x, 48_000, ch).unwrap_or((f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY));
        let clips = x.iter().any(|v| v.abs() >= 1.0);
        let studio = p.authentic().then(|| ui::export::render_channels(&project_for(p, false)).0);
        let same_on_studio = studio.as_ref().map(|y| ui::render::hash(y) == ui::render::hash(&x));
        let residual = studio.as_ref().map(|y| {
            let n = x.len().min(y.len());
            let e: f64 = (0..n).map(|k| ((x[k] - y[k]) as f64).powi(2)).sum::<f64>();
            let sx: f64 = x[..n].iter().map(|v| (*v as f64).powi(2)).sum::<f64>();
            let ly = ui::game_export::measure_ch(y, 48_000, ch).map_or(f64::NAN, |m| m.0);
            (10.0 * (e.max(1e-30) / sx.max(1e-30)).log10(), ly - lufs)
        });
        if write {
            let path = out.join(rel_path(p));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            // The pack keeps the sound, not the silence after it: cut 20 ms
            // after the last sample above -70 dBFS (the repo carries ~200 files).
            let floor = 10f32.powf(-70.0 / 20.0);
            let last = x.iter().rposition(|v| v.abs() > floor).map_or(0, |k| k / ch as usize + 1);
            let end = ((last + 960) * ch as usize).min(x.len());
            ui::export::write_wav_ch(&x[..end], ch, &path).unwrap();
        }
        rows.push(Row { i, lufs, tp, peak, clips, secs: x.len() as f64 / ch as f64 / 48_000.0, same_on_studio, residual });
    }
    // Category targets: fixed from the first pass (the designs before any
    // gain), at the 35th percentile of the category's new presets, a level
    // the quieter voices (a 12.5 % pulse, the triangle) can reach; kept in
    // presets/levels.json so later passes aim at the same level.
    let saved: serde_json::Value = std::fs::read_to_string(root.join("presets/levels.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(serde_json::Value::Null);
    let mut cats: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for r in &rows {
        let p = &all[r.i];
        if !p.classic && r.lufs.is_finite() {
            cats.entry(category(p)).or_default().push(r.lufs);
        }
    }
    let target: BTreeMap<String, f64> = cats
        .into_iter()
        .map(|(k, mut v)| {
            if let Some(t) = saved["targets"][&k].as_f64() {
                return (k, t);
            }
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let m = v[((v.len() - 1) as f64 * 0.35).round() as usize];
            (k, (m * 10.0).round() / 10.0)
        })
        .collect();
    // The gains already folded in (the pass is iterated: each run refines them).
    let old: BTreeMap<String, f64> = serde_json::from_value(saved["gains"].clone()).unwrap_or_default();
    let mut gains = BTreeMap::new();
    let mut table = Vec::new();
    let (mut worst, mut n_within, mut n_new) = (0.0f64, 0, 0);
    for r in &rows {
        let p = &all[r.i];
        let cat = category(p);
        let t = target.get(&cat).copied().unwrap_or(r.lufs);
        let dev = r.lufs - t;
        if !p.classic {
            n_new += 1;
            if dev.abs() <= 1.5 {
                n_within += 1;
            }
            worst = worst.max(dev.abs());
            let had = old.get(&p.name).copied().unwrap_or(1.0);
            let g = if dev.abs() > 0.5 { (had * 10f64.powf(-dev / 20.0)).clamp(0.1, 4.0) } else { had };
            if (g - 1.0).abs() > 1e-6 {
                gains.insert(p.name.clone(), g);
            }
        }
        table.push(format!(
            "| {} | {} | {} | {:.1} | {:+.1} | {:+.1} | {:+.1} | {} | {} |",
            p.name,
            cat,
            if p.classic { "classic" } else { "new" },
            r.lufs,
            dev,
            r.tp,
            r.peak,
            if r.clips { "**CLIPS**" } else { "no" },
            match (r.same_on_studio, r.residual) {
                (Some(true), _) => "identical".to_string(),
                (Some(false), Some((d, l))) => format!("residual {d:.1} dB, {l:+.2} LU"),
                _ => "–".to_string(),
            }
        ));
    }
    let clipping: Vec<&str> = rows.iter().filter(|r| r.clips).map(|r| all[r.i].name.as_str()).collect();
    let authentic = rows.iter().filter(|r| r.same_on_studio.is_some()).count();
    let identical = rows.iter().filter(|r| r.same_on_studio == Some(true)).count();
    let summary = format!(
        "{} presets ({} new, {} classic); clipping: {}; new presets within ±1.5 LU of their category's target: {n_within} of {n_new} (largest deviation {worst:.1} LU); authentic presets identical on a studio voice: {identical} of {authentic}",
        all.len(),
        n_new,
        all.len() - n_new,
        if clipping.is_empty() { "none".to_string() } else { clipping.join(", ") }
    );
    println!("{summary}");
    for (k, v) in &target {
        println!("  target {k}: {v:.1} LUFS");
    }
    for r in rows.iter().filter(|r| r.residual.is_some()).take(50) {
        let (d, l) = r.residual.unwrap();
        println!("  studio vs locked: {} residual {d:.1} dB, {l:+.2} LU", all[r.i].name);
    }
    for r in &rows {
        let p = &all[r.i];
        let dev = r.lufs - target.get(&category(p)).copied().unwrap_or(r.lufs);
        if !p.classic && dev.abs() > 1.5 {
            println!("  off target: {} ({}) {:+.1} LU", p.name, category(p), dev);
        }
    }
    if !write {
        return;
    }
    let levels = serde_json::json!({ "targets": target, "gains": gains });
    // The gains are iterated (each pass refines them and `design_bank.py
    // --levels` folds them in): only an explicit level pass rewrites them
    // (Batch 5: a re-render alone left the file claiming gains nothing had
    // folded in).
    if std::env::args().any(|a| a == "--iterate") {
        std::fs::write(root.join("presets/levels.json"), serde_json::to_string_pretty(&levels).unwrap() + "\n").unwrap();
    }
    let targets: Vec<String> = target.iter().map(|(k, v)| format!("| {k} | {v:.1} |")).collect();
    let md = format!(
        "# The preset bank: levels\n\nWritten by `cargo run --release -p ui --example bank_pack -- --write`. Each preset plays its role's phrase at 150 BPM (48 kHz); authentic presets on a locked track (the console's mixer and filter), the others on a studio track. Loudness is EBU R128 integrated (LUFS), true peak in dBTP, sample peak in dBFS.\n\n**{summary}.**\n\nLevel matching: a category's target is fixed from the first pass at the 35th percentile of its new presets (a level the quieter voices can reach); `presets/levels.json` holds the gains `design_bank.py --levels` folded in. The classic presets (Batch 1) are listed but not changed (RUNLOG PB-D6), except Music Box, Pizzicato and Menu Blip, raised in Batch 5 A2 as far as their voice allows (RUNLOG B5-D4).\n\n| Category | Target (LUFS) |\n|---|---|\n{}\n\n| Preset | Category | Bank | Loudness (LUFS) | vs target (LU) | True peak | Sample peak | Clips | Authentic on a studio voice |\n|---|---|---|---|---|---|---|---|---|\n{}\n",
        targets.join("\n"),
        table.join("\n")
    );
    std::fs::write(out.join("levels.md"), md).unwrap();
    // The listening page: grouped by folder, one line each.
    let mut by_folder: BTreeMap<(usize, String), Vec<&Row>> = BTreeMap::new();
    let order = ["Sounds", "Drums", "Hardware"];
    for r in &rows {
        let p = &all[r.i];
        let k = order.iter().position(|o| *o == p.path[0]).unwrap_or(9);
        by_folder.entry((k, p.folder())).or_default().push(r);
    }
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Cartwave preset bank: listening pack</title>\
<style>body{font:14px/1.45 system-ui,sans-serif;background:#121212;color:#e6e6e6;margin:0 auto;max-width:1100px;padding:24px}h1{font-size:22px}h2{font-size:15px;margin:28px 0 8px;color:#e8dcc0;border-bottom:1px solid #2a2a2a;padding-bottom:4px}\
table{border-collapse:collapse;width:100%}td{padding:4px 8px;border-bottom:1px solid #222;vertical-align:middle}td.n{font-weight:600;white-space:nowrap}td.d{color:#b0b0b0}td.t{color:#8a8a8a;font-size:12px;white-space:nowrap}audio{height:28px;width:260px}.lock{color:#e8dcc0}</style></head><body>\
<h1>Cartwave preset bank: listening pack</h1><p>For Mika and the CEO. Each preset plays a short phrase in its role at 150 BPM; authentic presets (🔒) play on a locked channel under their hardware profile. Every preset designed from scratch by Jeffrey Scott Frederick with Cartwave's macros (see <code>presets/PROVENANCE.md</code>). Levels: <a href=\"levels.md\">levels.md</a>.</p>",
    );
    html.push_str(&format!("<p>{}</p>", html_escape(&summary)));
    for ((_, folder), rs) in &by_folder {
        html.push_str(&format!("<h2>{} <span class=\"t\">({})</span></h2><table>", html_escape(folder), rs.len()));
        for r in rs {
            let p = &all[r.i];
            let lock = if p.authentic() { "<span class=\"lock\">🔒</span> " } else { "" };
            html.push_str(&format!(
                "<tr><td class=\"n\">{lock}{}</td><td class=\"d\">{}</td><td class=\"t\">{} · {:.1} LUFS</td><td><audio controls preload=\"none\" src=\"{}\"></audio></td></tr>",
                html_escape(&p.name),
                html_escape(&p.blurb),
                bank::voice_name(p.voice),
                r.lufs,
                rel_path(p)
            ));
        }
        html.push_str("</table>");
    }
    html.push_str("</body></html>\n");
    std::fs::write(out.join("index.html"), html).unwrap();
    // Provenance: one entry per preset (G14).
    let mut prov = String::from(
        "# Preset provenance\n\nOne entry per factory preset (PRESETS_PROMPT.md §7.4, VISION.md \"Presets\"; linked from `docs/ASSETS.md`). Every preset is **designed from scratch** with Cartwave's own macros (volume, arpeggio and its speed, pitch and duty sequences, vibrato, the channel controls, and on studio presets Unison / Detune / Drift). **Nothing is copied from games, ROMs, ripped patches or any third-party library**, and no name refers to a game, console maker or brand. Author of every preset: **Jeffrey Scott Frederick**. Licence: MIT OR Apache-2.0, as the code.\n\n- **New bank** (2026-11-27): generated by [`presets/tools/design_bank.py`](tools/design_bank.py) into [`presets/bank.json`](bank.json), level-matched with `crates/ui/examples/bank_pack.rs` (gains in `presets/levels.json`).\n- **Classic presets** (2026-11-25, Batch 1): the 60 hand-written presets in `crates/chip/src/presets.rs`, folded into the bank in each role's Classic folder (RUNLOG PB-D5), unchanged except where a dated change line says so.\n\nWritten by `cargo run --release -p ui --example bank_pack -- --write`.\n\n| # | Preset | Folder | Voice | Authentic | Author | Date | Origin | Generated by | Tags | Render |\n|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for (k, p) in all.iter().enumerate() {
        let auth = match p.profile {
            Some(SoundChip::Base2A03) => "🔒 2A03",
            Some(SoundChip::WithVrc6) => "🔒 VRC6",
            _ => "–",
        };
        prov.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | designed from scratch{} | `{}` | {} | [wav](../docs/audio/presets/{}) |\n",
            k + 1,
            p.name,
            p.folder(),
            bank::voice_name(p.voice),
            auth,
            p.author,
            p.date,
            change_line(&p.name),
            p.tool,
            p.tags.join(", "),
            rel_path(p)
        ));
    }
    std::fs::write(root.join("presets/PROVENANCE.md"), prov).unwrap();
    let total: f64 = rows.iter().map(|r| r.secs).sum();
    println!("wrote {} WAVs ({total:.0} s), index.html, levels.md, presets/levels.json, presets/PROVENANCE.md", rows.len());
}
