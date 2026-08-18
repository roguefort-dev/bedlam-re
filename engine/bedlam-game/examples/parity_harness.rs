//! Headless parity harness v0 (PLAN sec 6 P4, CPU half only; D24
//! defers the GPU/device half - this file touches neither wgpu nor
//! cpal).
//!
//! Drives GameHost end-to-end over a recorded input script and emits a
//! JSON report: the per-tick scene_hash chain (the post-frame scene
//! hash pushed once per executed tick - the crate rate-identity
//! sampling of D26), the final canonical
//! Frame parity hash, the final sim state hash and the audio mix
//! stream hash. Purpose: a deterministic CPU baseline the P4
//! wine/DOSBox runtime comparisons can diff against.
//!
//! The bedlam-game crate stays hermetic: every byte of game data
//! crosses through the ByteSource implemented here (fs is allowed in
//! an example, never in the crate).
//!
//! Input script grammar (text, one command per line, # comments):
//!   step <frames> [buttons] [mouse_buttons] [dx] [dy]
//!       pump the given input for <frames> host frames; every numeric
//!       argument parses as hex with 0x prefix, else decimal
//!   act <Advance|Back|Options|MissionComplete|MissionFail|Quit>
//!       host-applied scene intent (GameHost::apply)
//!   music <NAME.MRS>
//!       load a music track through the byte source now (the next
//!       scene sync attaches it)
//!
//! Usage: parity_harness [--root DIR] [--script PATH] [--out PATH] [--dt N]
//!   --root   install tree (default: repo game-data/BEDLAM)
//!   --script input script (default: the embedded walk below)
//!   --out    report file (default: stdout)
//!   --dt     host dt in 240 Hz subticks per frame (default 4 = 60 Hz)

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use bedlam_assets::music::parse_mrw;
use bedlam_core::hash::Fnv1a64;
use bedlam_core::input::InputFrame;
use bedlam_core::sim::SimConfig;
use bedlam_game::{ByteSource, GameConfig, GameError, GameHost, SceneAction, OPTIONS_NAME};
use bedlam_render::Vga6;

/// Host audio pull per frame: 11025 Hz native (DESIGN-AUDIO) over
/// 60 Hz frames. The mixer Q16 grid keeps event timing exact; the
/// pull rate is host business (D17 bucket b), pinned here so the
/// stream hash is reproducible. 184 = ceil(11025/60), even length.
const SAMPLES_PER_FRAME: usize = 184;

/// Native mixer rate (both builds, DESIGN-AUDIO).
const AUDIO_RATE: u32 = 11025;

/// 240 Hz sub-tick grid (bedlam-core frame.rs).
const SUBTICKS_PER_TICK: u32 = 4;

/// Filesystem byte source rooted at the install tree. Resolves
/// top-level names first, then SOUND/MIDI (where the shipped .MRS
/// files live).
struct FsSource {
    root: PathBuf,
}

impl ByteSource for FsSource {
    fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
        let candidates = [
            self.root.join(name),
            self.root.join("SOUND").join("MIDI").join(name),
        ];
        for path in candidates {
            if path.is_file() {
                return fs::read(&path).map_err(|e| GameError::AssetMissing {
                    name: format!("{name}: {e}"),
                });
            }
        }
        Err(GameError::AssetMissing {
            name: name.to_string(),
        })
    }
}

/// One parsed script command.
#[derive(Debug)]
enum Cmd {
    Step { frames: u32, input: InputFrame },
    Act(SceneAction),
    Music(String),
}

fn parse_script(text: &str) -> Result<Vec<Cmd>, String> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("#") {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let bad = |msg: &str| format!("line {}: {}", i + 1, msg);
        match words[0] {
            "step" => {
                if words.len() < 2 || words.len() > 6 {
                    return Err(bad("step needs 1..=5 args"));
                }
                let num = |w: &str| -> Result<i64, String> {
                    if let Some(hex) = w.strip_prefix("0x") {
                        i64::from_str_radix(hex, 16).map_err(|_| bad("bad hex"))
                    } else {
                        w.parse().map_err(|_| bad("bad number"))
                    }
                };
                let frames: u32 =
                    u32::try_from(num(words[1])?).map_err(|_| bad("frames overflow"))?;
                let buttons = if words.len() > 2 {
                    u32::try_from(num(words[2])?).map_err(|_| bad("buttons overflow"))?
                } else {
                    0
                };
                let mouse_buttons = if words.len() > 3 {
                    u8::try_from(num(words[3])?).map_err(|_| bad("mouse overflow"))?
                } else {
                    0
                };
                let dx = if words.len() > 4 {
                    i16::try_from(num(words[4])?).map_err(|_| bad("dx overflow"))?
                } else {
                    0
                };
                let dy = if words.len() > 5 {
                    i16::try_from(num(words[5])?).map_err(|_| bad("dy overflow"))?
                } else {
                    0
                };
                out.push(Cmd::Step {
                    frames,
                    input: InputFrame {
                        buttons,
                        mouse_dx: dx,
                        mouse_dy: dy,
                        mouse_buttons,
                    },
                });
            }
            "act" => {
                let action = match words.get(1).copied().unwrap_or("") {
                    "Advance" => SceneAction::Advance,
                    "Back" => SceneAction::Back,
                    "Options" => SceneAction::Options,
                    "MissionComplete" => SceneAction::MissionComplete,
                    "MissionFail" => SceneAction::MissionFail,
                    "Quit" => SceneAction::Quit,
                    other => return Err(bad(&format!("unknown action {other}"))),
                };
                out.push(Cmd::Act(action));
            }
            "music" => {
                let name = words.get(1).copied().unwrap_or("");
                if name.is_empty() {
                    return Err(bad("music needs a file name"));
                }
                out.push(Cmd::Music(name.to_string()));
            }
            other => return Err(bad(&format!("unknown command {other}"))),
        }
    }
    Ok(out)
}

/// Embedded default script: boot, every track scene with its shipped
/// .MRS, the mission lifecycle, and one real mouse click (Title ->
/// Brief) so the input-edge path (D26) runs alongside host intents.
const DEFAULT_SCRIPT: &str = "\
# boot countdown + title idle (silence baseline)
step 40
# Options screen and its track
music OPTIONS.MRS
act Options
step 90
act Back
step 15
# back to Title, then into Brief by a REAL mouse click (D26 edge)
music BRIEF.MRS
step 1 0 1 0 0
step 89
# Select with its own track
music SELECT.MRS
act Advance
step 90
# Mission (no scripted track)
act Advance
step 45
# Debrief track after a completed mission
music DEBRIEF.MRS
act MissionComplete
step 90
# zone tail: stage-1 full-mask = 1, so the FIRST completion is
# zone-complete -> Debrief + Advance -> Cutscene (SHOP.MRS loads
# here; it attaches when the Shop scene is entered below)
music SHOP.MRS
act Advance
step 60
act Advance
step 30
# second lap at stage 2 (full-mask 0xf, mask 1): NOT zone-complete,
# so Debrief + Advance lands in Shop - the SHOP track plays
act Advance
step 45
act MissionComplete
step 90
act Advance
step 60
act Advance
step 30
";

/// Minimal JSON string escaping (quotes, backslash, control bytes).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        let code = c as u32;
        if code < 0x20 {
            out.push_str(&format!("\\u{code:04x}"));
        } else if code == 0x22 {
            out.push_str("\\\"");
        } else if code == 0x5c {
            out.push_str("\\\\");
        } else {
            out.push(c);
        }
    }
    out
}

/// 0x-prefixed 16-hex-digit form (stable, unsigned).
fn hex16(v: u64) -> String {
    format!("{v:#018x}")
}

struct Args {
    root: PathBuf,
    script: Option<PathBuf>,
    out: Option<PathBuf>,
    dt: u32,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        root: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM"),
        script: None,
        out: None,
        dt: SUBTICKS_PER_TICK,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let value = it
            .next()
            .ok_or_else(|| format!("flag {flag} needs a value"))?;
        match flag.as_str() {
            "--root" => args.root = PathBuf::from(value),
            "--script" => args.script = Some(PathBuf::from(value)),
            "--out" => args.out = Some(PathBuf::from(value)),
            "--dt" => {
                args.dt = value.parse().map_err(|_| format!("bad dt {value}"))?;
            }
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok(args)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;
    let script_text = match &args.script {
        Some(path) => fs::read_to_string(path)
            .map_err(|e| format!("cannot read script {}: {e}", path.display()))?,
        None => DEFAULT_SCRIPT.to_string(),
    };
    let cmds = parse_script(&script_text).map_err(|e| format!("script error: {e}"))?;

    // Assets, config, host.
    let mut source = FsSource {
        root: args.root.clone(),
    };
    let config = GameConfig::load(&mut source)?;
    let mut assets: Vec<(String, usize, u64)> = Vec::new();
    let record = |assets: &mut Vec<(String, usize, u64)>, name: &str, bytes: &[u8]| {
        let hash = bedlam_core::hash::fnv1a64(bytes);
        match assets.iter_mut().find(|(n, _, _)| n == name) {
            Some(entry) => {
                entry.2 = hash;
            }
            None => assets.push((name.to_string(), bytes.len(), hash)),
        }
    };
    record(&mut assets, OPTIONS_NAME, &source.load(OPTIONS_NAME)?);
    let palette: [Vga6; 256] = [[0u8, 0, 0]; 256];
    let mut host = GameHost::new(&config, &SimConfig::default(), palette);

    // Pump state.
    let mut per_tick: Vec<u64> = Vec::new();
    let mut transitions: Vec<(u32, String, String)> = Vec::new();
    let mut chain = Fnv1a64::new();
    let mut audio_hash = Fnv1a64::new();
    let mut audio_samples = 0usize;
    let mut waves_loaded = 0usize;
    let mut audio_nonzero = 0usize;
    let mut frames = 0u32;
    let mut last_scene = host.scene();
    let mut audio_buf = [0i16; SAMPLES_PER_FRAME * 2];

    for cmd in &cmds {
        match cmd {
            Cmd::Music(name) => {
                let bytes = source.load(name)?;
                record(&mut assets, name, &bytes);
                host.load_music(&bytes)?;
                // Sibling .MRW instrument bank (RE-EXW-MUSIC:
                // mrw_load pairs with load_midi; 11025 Hz 8-bit
                // mono waves, one slot per directory entry).
                if let Some(stem) = name.strip_suffix(".MRS") {
                    let bank_name = format!("{stem}.MRW");
                    let bank_bytes = source.load(&bank_name)?;
                    record(&mut assets, &bank_name, &bank_bytes);
                    let bank = parse_mrw(&bank_bytes)?;
                    for i in 0..bank.count {
                        if let Some((a, b)) = bank.wave_range(i) {
                            host.mixer_mut().load_wave(i as u16, &bank_bytes[a..b])?;
                            waves_loaded += 1;
                        }
                    }
                }
            }
            Cmd::Act(action) => {
                host.apply(*action);
                let now = host.scene();
                if now != last_scene {
                    transitions.push((frames, format!("{last_scene:?}"), format!("{now:?}")));
                    last_scene = now;
                }
            }
            Cmd::Step { frames: n, input } => {
                for _ in 0..*n {
                    let executed = host.pump_frame(args.dt, input);
                    for _ in 0..executed {
                        let h = host.scene_hash().0;
                        per_tick.push(h);
                        chain.write_u64(h);
                    }
                    let now = host.scene();
                    if now != last_scene {
                        transitions.push((frames, format!("{last_scene:?}"), format!("{now:?}")));
                        last_scene = now;
                    }
                    let written = host.render_audio(&mut audio_buf)?;
                    for s in &audio_buf[..written * 2] {
                        audio_hash.write_i16(*s);
                        if *s != 0 {
                            audio_nonzero += 1;
                        }
                    }
                    audio_samples += written;
                    frames += 1;
                }
            }
        }
    }

    // Report.
    let sim = host.driver().sim();
    let frame = host.frame();
    let script_label = match &args.script {
        Some(p) => p.display().to_string(),
        None => "embedded".to_string(),
    };
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"bedlam-parity-harness/0\",\n");
    json.push_str(&format!(
        "  \"meta\": {{ \"seed\": {}, \"dt_subticks\": {}, \"subtick_hz\": 240, \"audio_rate\": {}, \"samples_per_frame\": {}, \"frames\": {}, \"ticks\": {}, \"script\": \"{}\" }},\n",
        SimConfig::default().seed,
        args.dt,
        AUDIO_RATE,
        SAMPLES_PER_FRAME,
        frames,
        per_tick.len(),
        json_escape(&script_label)
    ));
    json.push_str("  \"assets\": {\n");
    for (i, (name, len, hash)) in assets.iter().enumerate() {
        json.push_str(&format!(
            "    \"{}\": {{ \"bytes\": {}, \"fnv1a64\": \"{}\" }}{}\n",
            json_escape(name),
            len,
            hex16(*hash),
            if i + 1 < assets.len() { "," } else { "" }
        ));
    }
    json.push_str("  },\n");
    json.push_str(&format!(
        "  \"config\": {{ \"player_name\": \"{}\", \"volume\": {}, \"music_master\": {}, \"language\": {} }},\n",
        json_escape(&config.player_name),
        config.volume,
        config.music_master(),
        config.language
    ));
    json.push_str(&format!(
        "  \"scene\": {{ \"final\": \"{:?}\", \"final_hash\": \"{}\", \"chain_fnv1a64\": \"{}\",\n    \"transitions\": [",
        host.scene(),
        hex16(host.scene_hash().0),
        hex16(chain.finish())
    ));
    for (i, (f, from, to)) in transitions.iter().enumerate() {
        json.push_str(&format!(
            "{}[{}, \"{}\", \"{}\"]",
            if i == 0 { "" } else { ", " },
            f,
            json_escape(from),
            json_escape(to)
        ));
    }
    json.push_str("],\n    \"per_tick\": [");
    for (i, h) in per_tick.iter().enumerate() {
        json.push_str(&format!(
            "{}\"{}\"",
            if i == 0 { "" } else { "," },
            hex16(*h)
        ));
    }
    json.push_str("] },\n");
    json.push_str(&format!(
        "  \"sim\": {{ \"tick_index\": {}, \"state_hash\": \"{}\" }},\n",
        sim.tick_index(),
        hex16(sim.state_hash().0)
    ));
    json.push_str(&format!(
        "  \"frame\": {{ \"parity_hash\": \"{}\", \"palette_dirty\": {} }},\n",
        hex16(frame.parity_hash()),
        frame.palette_dirty
    ));
    json.push_str(&format!(
        "  \"audio\": {{ \"samples\": {}, \"stream_fnv1a64\": \"{}\", \"nonzero_samples\": {}, \"waves\": {} }}\n",
        audio_samples,
        hex16(audio_hash.finish()),
        audio_nonzero,
        waves_loaded
    ));
    json.push_str("}\n");

    match &args.out {
        Some(path) => fs::write(path, &json)?,
        None => print!("{json}"),
    }
    eprintln!(
        "parity_harness: {} frames, {} ticks, scene {:?}, audio {} samples ({} nonzero)",
        frames,
        per_tick.len(),
        host.scene(),
        audio_samples,
        audio_nonzero
    );
    Ok(())
}
