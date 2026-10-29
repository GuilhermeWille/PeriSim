//! Persistência local. Um salvamento guarda apenas (ID do caso, versão do gerador,
//! lista de comandos). Como o motor é determinístico, o estado é reconstruído por
//! replay — o que também revalida a auditoria encadeada a cada carregamento.

use pericia_core::casegen::{generate, parse_case_id, GEN_VERSION};
use pericia_core::engine::Sim;
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("PERICIA_DATA") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into());
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(&home))
    } else if cfg!(target_os = "macos") {
        PathBuf::from(&home).join("Library/Application Support")
    } else {
        std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(&home).join(".local/share"))
    };
    base.join("pericia-sim")
}

fn ensure(p: &PathBuf) {
    let _ = std::fs::create_dir_all(p);
}

pub struct SaveGame {
    pub case_id: String,
    pub gen_version: String,
    pub commands: Vec<String>,
}

pub fn save_current(sim: &Sim, commands: &[String]) -> Result<PathBuf, String> {
    let dir = data_dir().join("saves");
    ensure(&dir);
    let path = dir.join("atual.json");
    let v = json!({
        "case_id": sim.case.id,
        "gen_version": sim.case.gen_version,
        "commands": commands,
        "audit_last_hash": sim.audit.last_hash(),
    });
    std::fs::write(&path, serde_json::to_string_pretty(&v).unwrap()).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn load_current() -> Option<SaveGame> {
    let s = std::fs::read_to_string(data_dir().join("saves/atual.json")).ok()?;
    let v: Value = serde_json::from_str(&s).ok()?;
    Some(SaveGame {
        case_id: v["case_id"].as_str()?.to_string(),
        gen_version: v["gen_version"].as_str().unwrap_or("").to_string(),
        commands: v["commands"].as_array()?.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(),
    })
}

/// Reconstrói a simulação por replay. Falha se a versão do gerador mudou.
pub fn replay(sg: &SaveGame) -> Result<Sim, String> {
    if sg.gen_version != GEN_VERSION {
        return Err(format!("Salvamento criado com o gerador v{} (atual v{}); o caso não pode ser reproduzido com fidelidade.", sg.gen_version, GEN_VERSION));
    }
    let (m, d, seed) = parse_case_id(&sg.case_id).ok_or("ID de caso inválido no salvamento")?;
    let mut sim = Sim::new(generate(m, d, seed));
    for c in &sg.commands {
        sim.exec(c);
    }
    sim.audit.verify().map_err(|seq| format!("Auditoria inconsistente na entrada {}", seq))?;
    Ok(sim)
}

#[derive(Clone)]
pub struct HistoryEntry {
    pub id: String,
    pub module: String,
    pub difficulty: u8,
    pub score: u32,
    pub max: u32,
    pub verdict: String,
    pub when: String,
}

pub fn history() -> Vec<HistoryEntry> {
    let Ok(s) = std::fs::read_to_string(data_dir().join("historico.json")) else { return vec![] };
    let Ok(v) = serde_json::from_str::<Value>(&s) else { return vec![] };
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|e| HistoryEntry {
                    id: e["id"].as_str().unwrap_or("").into(),
                    module: e["module"].as_str().unwrap_or("").into(),
                    difficulty: e["difficulty"].as_u64().unwrap_or(1) as u8,
                    score: e["score"].as_u64().unwrap_or(0) as u32,
                    max: e["max"].as_u64().unwrap_or(100) as u32,
                    verdict: e["verdict"].as_str().unwrap_or("").into(),
                    when: e["when"].as_str().unwrap_or("").into(),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn push_history(e: HistoryEntry) {
    let dir = data_dir();
    ensure(&dir);
    let mut all: Vec<Value> = std::fs::read_to_string(dir.join("historico.json")).ok().and_then(|s| serde_json::from_str::<Vec<Value>>(&s).ok()).unwrap_or_default();
    all.retain(|x| x["id"].as_str() != Some(&e.id));
    all.push(json!({"id": e.id, "module": e.module, "difficulty": e.difficulty, "score": e.score, "max": e.max, "verdict": e.verdict, "when": e.when}));
    let _ = std::fs::write(dir.join("historico.json"), serde_json::to_string_pretty(&all).unwrap());
}

pub fn export(sim: &Sim) -> Result<(PathBuf, PathBuf), String> {
    let dir = data_dir().join("exportados");
    ensure(&dir);
    let pdf = dir.join(format!("{}.pdf", sim.case.id));
    let md = dir.join(format!("{}.md", sim.case.id));
    std::fs::write(&pdf, pericia_core::pdf::document_pdf(sim)).map_err(|e| e.to_string())?;
    std::fs::write(&md, pericia_core::report::markdown(sim)).map_err(|e| e.to_string())?;
    Ok((pdf, md))
}

pub struct Settings {
    pub ui_scale: f32,
    pub render3d: bool,
    pub low_power: bool,
}

pub fn load_settings() -> Settings {
    let d = Settings { ui_scale: 1.0, render3d: true, low_power: true };
    let Ok(s) = std::fs::read_to_string(data_dir().join("config.json")) else { return d };
    let Ok(v) = serde_json::from_str::<Value>(&s) else { return d };
    Settings {
        ui_scale: v["ui_scale"].as_f64().unwrap_or(1.0) as f32,
        render3d: v["render3d"].as_bool().unwrap_or(true),
        low_power: v["low_power"].as_bool().unwrap_or(true),
    }
}

pub fn save_settings(s: &Settings) {
    let dir = data_dir();
    ensure(&dir);
    let _ = std::fs::write(dir.join("config.json"), json!({"ui_scale": s.ui_scale, "render3d": s.render3d, "low_power": s.low_power}).to_string());
}
