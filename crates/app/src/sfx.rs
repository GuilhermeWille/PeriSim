//! Desenho de som: que som toca em cada ação, onde (posição 3D) e com que volume.
//! Só lê o estado; nunca executa comandos nem altera a simulação.

use crate::audio::Cat;
use crate::{App, Area, Screen};
use macroquad::prelude::*;
use pericia_core::model::{ItemKind, Setting};

/// Som de uma ação pericial pelo texto do comando (primeiras palavras).
pub fn command_sound(cmd: &str) -> Option<&'static str> {
    let t: Vec<String> = cmd.split_whitespace().map(|x| x.to_lowercase()).collect();
    let a = t.first().map(|s| s.as_str()).unwrap_or("");
    let b = t.get(1).map(|s| s.as_str()).unwrap_or("");
    let last = t.last().map(|s| s.as_str()).unwrap_or("");
    Some(match (a, b) {
        ("fotografar", _) => "foto",
        ("lacre", "conferir") => "papel",
        ("lacre", "aplicar") => "lacre_aplicar",
        ("deslacrar", _) => "deslacrar",
        ("abrir", _) => "abrir_gabinete",
        ("componente", _) => "componente_remover",
        ("midia", _) => "midia_remover",
        ("bloqueio", _) => "bloqueio",
        ("imagem", _) | ("clone", _) | ("sanitizar", _) => "aquisicao",
        ("hash", _) | ("verificar-hash", _) => "hash",
        ("energia", _) => "energia_desligar",
        ("rede", "isolar") => "rede_isolar",
        ("memoria", _) | ("volume", _) => "digitacao",
        ("isolar", _) if last.starts_with("alum") => "aluminio",
        ("isolar", _) => "faraday",
        ("airplane", _) | ("alarmes", _) | ("tela", _) | ("desbloquear", _) | ("imei", _) => "celular_toque",
        ("carregar", _) => "celular_carregar",
        ("sim", _) => "sim_remover",
        ("extrair", _) => "extracao",
        ("embalar", _) => "embalar",
        ("etiquetar", _) | ("camera", _) => "etiquetar",
        ("inspecionar", _) => "inspecionar",
        ("senha", _) | ("solicitar-senhas", _) | ("anotar", _) => "caneta",
        ("bios", _) => "digitacao",
        ("pegar", _) => "pegar_ferramenta",
        ("guardar", _) | ("soltar", _) => "guardar_ferramenta",
        ("luvas", _) => "luvas",
        ("pulseira", _) => "pulseira",
        ("desparafusar", _) => "parafuso",
        ("cabo", _) => "cabo_desconectar",
        ("conectar", _) => "conectar",
        ("marcar", _) => "placa",
        ("comunicar", _) => "caneta",
        _ => return None,
    })
}

/// Posição no mundo do objeto citado no comando (item ou mídia de um item).
fn source_pos(app: &App, cmd: &str) -> Option<Vec3> {
    let sim = app.sim.as_ref()?;
    for tok in cmd.split_whitespace() {
        let up = tok.to_uppercase();
        if let Some(p) = app.item_pos.get(&up) {
            return Some(*p);
        }
        if let Some(it) = sim.case.items.iter().find(|i| i.media.iter().any(|m| *m == up)) {
            if let Some(p) = app.item_pos.get(&it.id) {
                return Some(*p);
            }
        }
    }
    None
}

/// Toca o som de um comando executado (terminal, painel, estação ou celular).
pub fn command(app: &mut App, cmd: &str, ok: bool) {
    if !ok {
        app.audio.play("ui_erro", Cat::Interface, 0.8);
        return;
    }
    let Some(name) = command_sound(cmd) else { return };
    // na estação o perito opera pelo computador: som próximo, sem posição
    if app.area == Area::Station || !app.settings.render3d {
        app.audio.play(name, Cat::Efeitos, 0.85);
        return;
    }
    match source_pos(app, cmd) {
        Some(p) => {
            let (ear, right) = app.ear;
            app.audio.play_at(name, Cat::Efeitos, 1.0, p, ear, right);
        }
        None => app.audio.play(name, Cat::Efeitos, 0.85),
    }
}

/// Som de tocar/selecionar um objeto na cena.
pub fn interact(app: &mut App, pick_id: &str, at: Vec3) {
    let name = if let Some(it) = app.sim.as_ref().and_then(|s| s.case.item(pick_id)) {
        match it.kind {
            ItemKind::Desktop | ItemKind::Servidor => "interagir_gabinete",
            ItemKind::Notebook => "interagir_notebook",
            ItemKind::Celular | ItemKind::Tablet => "interagir_celular",
            _ => "interagir_midia",
        }
    } else {
        "interagir_ferramenta"
    };
    let (ear, right) = app.ear;
    app.audio.play_at(name, Cat::Efeitos, 0.9, at, ear, right);
}

/// Passo do perito (chamado pelo controle em primeira pessoa a cada passada).
pub fn footstep(app: &mut App, running: bool) {
    if !app.settings.footsteps {
        return;
    }
    let local = app.sim.as_ref().map(|s| s.case.setting == Setting::Local).unwrap_or(false);
    let k = 1 + (app.step_n % 4);
    app.step_n += 1;
    let name = format!("passo_{}_{}", if local { "madeira" } else { "vinil" }, k);
    app.audio.play_var(&name, Cat::Efeitos, if running { 0.9 } else { 0.72 }, 0.05);
}

/// Atualização por quadro: ambiente do lugar, camada da estação, cliques e teclas.
pub fn update(app: &mut App) {
    let amb = match (app.screen, app.sim.as_ref()) {
        (Screen::Game, Some(s)) => Some(if s.case.setting == Setting::Local { "ambiente_escritorio" } else { "ambiente_laboratorio" }),
        _ => None,
    };
    app.audio.ambient(amb);
    app.audio.music_playlist(app.settings.music_on);
    let station = app.screen == Screen::Game && app.area == Area::Station;
    app.audio.layer(if station { Some("ambiente_estacao") } else { None });
    // cliques de botões e abas da interface
    if app.ui.clicks > 0 {
        if app.tablet.open {
            app.audio.play_var("tablet_toque", Cat::Interface, 1.0, 0.04);
        } else {
            app.audio.play_var("ui_clique", Cat::Interface, 0.7, 0.03);
        }
        app.ui.clicks = 0;
    }
    for n in std::mem::take(&mut app.tablet.sfx) {
        app.audio.play(n, Cat::Interface, 0.9);
    }
    // teclado: só quando há um campo de texto com foco (terminal, editor, anotação)
    if app.ui.focus.is_some() && (!app.ui.chars.is_empty() || app.ui.key_back || app.ui.key_enter) {
        app.key_n = app.key_n.wrapping_add(1);
        let name = format!("tecla_{}", 1 + app.key_n % 3);
        app.audio.play_var(&name, Cat::Interface, if app.ui.key_enter { 0.9 } else { 0.6 }, 0.06);
    }
}

#[cfg(test)]
mod t {
    use super::command_sound;

    #[test]
    fn mapeamento_de_comandos() {
        assert_eq!(command_sound("fotografar item-01"), Some("foto"));
        assert_eq!(command_sound("lacre conferir item-01"), Some("papel"));
        assert_eq!(command_sound("lacre aplicar item-02"), Some("lacre_aplicar"));
        assert_eq!(command_sound("isolar item-01 aluminio"), Some("aluminio"));
        assert_eq!(command_sound("isolar item-01 faraday"), Some("faraday"));
        assert_eq!(command_sound("rede isolar item-02"), Some("rede_isolar"));
        assert_eq!(command_sound("rede logs"), None, "consultas de rede na estação não têm som físico");
        assert_eq!(command_sound("fs ls /"), None);
        assert_eq!(command_sound("IMAGEM M-01"), Some("aquisicao"));
    }

    #[test]
    fn todo_som_mapeado_existe_no_banco() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sounds");
        let cmds = [
            "fotografar x", "lacre conferir x", "lacre aplicar x", "deslacrar x", "abrir x", "componente remover x y", "midia remover x y", "bloqueio m",
            "imagem m", "verificar-hash m", "energia desligar x", "rede isolar x", "memoria x", "isolar x aluminio", "isolar x faraday", "airplane x",
            "carregar x", "sim remover x 1", "extrair x", "embalar x", "etiquetar x y", "inspecionar local", "senha",
        ];
        for c in cmds {
            let n = command_sound(c).unwrap();
            assert!(dir.join(format!("{}.wav", n)).is_file(), "falta {}.wav ({})", n, c);
        }
        for c in ["pegar camera", "guardar", "luvas calcar", "pulseira colocar", "desparafusar item-01", "cabo desconectar x y dados", "conectar m-01 bloqueador", "marcar v-01", "comunicar v-01"] {
            let n = command_sound(c).unwrap();
            assert!(dir.join(format!("{}.wav", n)).is_file(), "falta {}.wav ({})", n, c);
        }
        let mut extra = vec!["ui_clique", "ui_erro", "tablet_sacar", "tablet_guardar", "tablet_toque", "nota_gravada", "cadeira", "ambiente_laboratorio", "ambiente_escritorio", "ambiente_estacao", "musica_noir", "musica_noir_2", "musica_noir_3", "maleta_abrir", "maleta_fechar"];
        extra.extend(["interagir_gabinete", "interagir_notebook", "interagir_celular", "interagir_midia", "interagir_ferramenta"]);
        for n in extra {
            assert!(dir.join(format!("{}.wav", n)).is_file(), "falta {}.wav", n);
        }
        for k in 1..=4 {
            assert!(dir.join(format!("passo_vinil_{}.wav", k)).is_file() && dir.join(format!("passo_madeira_{}.wav", k)).is_file());
        }
    }
}
