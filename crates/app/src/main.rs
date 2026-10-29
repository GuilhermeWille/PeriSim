//! Simulador de Informática Forense — interface.
//! Duas áreas: Área de Hardware (cena 3D com os equipamentos e ações físicas) e
//! Estação Pericial (um PC na cena 3D cujo monitor abre o "PeritOS", com software/rede).

mod desktop;
mod linuxde;
mod save;
mod assets;
mod fps;
mod phone;
mod render;
mod tablet;
mod audio;
mod config;
mod sfx;
mod video;
mod world;
mod ui;

use desktop::{DApp, DesktopState};
use macroquad::prelude::*;
use pericia_core::casegen::{generate, parse_case_id};
use pericia_core::engine::Sim;
use pericia_core::model::*;
use ui::*;

fn conf() -> macroquad::conf::Conf {
    // PERICIA_WINDOW=LxA (ex.: 1920x1080) — tamanho inicial da janela, para testar resoluções
    let (ww, wh) = std::env::var("PERICIA_WINDOW")
        .ok()
        .and_then(|v| v.split_once('x').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))))
        .unwrap_or((1280, 760));
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Perícia Sim — Simulador de Informática Forense".into(),
            window_width: ww,
            window_height: wh,
            high_dpi: false,
            sample_count: 1,
            window_resizable: true,
            ..Default::default()
        },
        // lotes grandes (modelos 3D): cada malha do renderizador cabe numa chamada de desenho
        draw_call_vertex_capacity: render::MAX_BATCH_VERTS,
        draw_call_index_capacity: render::MAX_BATCH_INDICES,
        ..Default::default()
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Screen {
    Menu,
    Game,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Area {
    Hardware,
    Station,
}

pub struct Term {
    pub lines: String,
    pub input: String,
    pub scroll: f32,
    pub history: Vec<String>,
    pub hist_idx: usize,
}

pub struct App {
    pub ui: Ui,
    pub screen: Screen,
    pub area: Area,
    pub sim: Option<Sim>,
    pub cmdlog: Vec<String>,
    pub term: Term,
    pub player: fps::Player,
    pub renderer: Option<render::Renderer>,
    pub hover_pick: Option<String>,
    pub seat: (Vec3, f32),
    pub selected: Option<String>,
    pub hover_label: Option<String>,
    pub desk: DesktopState,
    pub station_zoom: bool,
    pub settings: save::Settings,
    pub menu_module: ModuleId,
    pub menu_diff: u8,
    pub menu_seed: String,
    pub menu_msg: String,
    pub panel_scroll: f32,
    pub panel_input: String,
    pub term_open: bool,
    pub toast: Option<(String, f64)>,
    pub history: Vec<save::HistoryEntry>,
    pub show_help_overlay: bool,
    pub de: linuxde::LinuxDe,
    pub phone: phone::PhoneUi,
    /// Tablet do perito (missão, objeto, apoio, anotações) — tecla T na área de hardware.
    pub tablet: tablet::TabletUi,
    pub audio: audio::Audio,
    /// Ouvinte (posição da câmera, vetor à direita) para os sons posicionados.
    pub ear: (Vec3, Vec3),
    /// Posição no mundo de cada item visível (atualizada a cada quadro da cena 3D).
    pub item_pos: std::collections::HashMap<String, Vec3>,
    pub step_n: u32,
    pub key_n: u32,
    pub config: config::ConfigUi,
    /// Legenda exibida em roteiros de demonstração (gravação de vídeo).
    pub caption: Option<String>,
}

impl App {
    /// Executa um comando no motor, registra no terminal e no log de replay, e salva.
    pub fn run(&mut self, cmd: &str) -> String {
        let Some(sim) = &mut self.sim else { return String::new() };
        let out = sim.exec(cmd);
        self.cmdlog.push(cmd.to_string());
        self.term.lines.push_str(&format!("\n> {}\n{}\n", cmd, if out.ok { out.text.clone() } else { format!("ERRO: {}", out.text) }));
        self.term.scroll = f32::MAX;
        let _ = save::save_current(self.sim.as_ref().unwrap(), &self.cmdlog);
        sfx::command(self, cmd, out.ok);
        if out.ok { out.text } else { format!("ERRO: {}", out.text) }
    }
    /// Como `run`, mas sem ecoar a saída completa no terminal (edições do documento).
    pub fn run_quiet(&mut self, cmd: &str) {
        let Some(sim) = &mut self.sim else { return };
        let out = sim.exec(cmd);
        self.cmdlog.push(cmd.to_string());
        let shown: String = cmd.chars().take(70).collect();
        self.term.lines.push_str(&format!("\n> {}{}\n{}\n", shown, if cmd.chars().count() > 70 { "…" } else { "" }, out.text));
        let _ = save::save_current(self.sim.as_ref().unwrap(), &self.cmdlog);
    }
    pub fn nav_message(&mut self, cmd: &str) {
        let out = self.run(cmd);
        self.desk.nav_view = out;
    }
    pub fn toast(&mut self, t: &str) {
        self.toast = Some((t.to_string(), get_time() + 4.0));
    }
    pub fn finish_case(&mut self, out: String) {
        self.desk.eval_text = out.clone();
        if let Some(s) = &self.sim {
            let ev = pericia_core::assess::evaluate(s);
            let (got, max, destroyed) = pericia_core::assess::total(&ev, s);
            let verdict = if destroyed { "Reprovado (perda de evidência)" } else if got * 100 / max.max(1) >= 70 { "Aprovado" } else { "Reprovado" };
            save::push_history(save::HistoryEntry {
                id: s.case.id.clone(),
                module: s.case.module.code().into(),
                difficulty: s.case.difficulty,
                score: got,
                max,
                verdict: verdict.into(),
                when: pericia_core::util::fmt_dt(s.clock, pericia_core::util::BRT),
            });
            let _ = save::export(s);
        }
        self.history = save::history();
        self.desk.app = DApp::Avaliacao;
        self.desk.scroll = 0.0;
    }

    pub fn start_case(&mut self, m: ModuleId, d: u8, seed: u32) {
        let case = generate(m, d, seed);
        let mut sim = Sim::new(case);
        sim.actor = "Perito(a) em treinamento".into();
        self.term.lines = format!(
            "PeritOS — terminal do simulador. Digite 'ajuda' para os comandos.\nCaso {} carregado (semente {}, gerador v{}).\n",
            sim.case.id, sim.case.seed, sim.case.gen_version
        );
        self.sim = Some(sim);
        self.cmdlog.clear();
        self.screen = Screen::Game;
        self.area = Area::Hardware;
        self.selected = None;
        self.desk = DesktopState::new();
        self.station_zoom = false;
        self.player = fps::Player::new();
        self.player.case.clear();
        self.phone.close();
        let cid = self.sim.as_ref().unwrap().case.id.clone();
        self.tablet.bind_case(&cid, true);
        let _ = save::save_current(self.sim.as_ref().unwrap(), &self.cmdlog);
        // o resumo do caso aparece primeiro no terminal
        let brief = pericia_core::tools::describe_case(self.sim.as_ref().unwrap());
        self.term.lines.push_str(&format!("\n{}\n", brief));
        self.term.scroll = 0.0;
    }

    pub fn resume(&mut self) {
        match save::load_current().map(|sg| save::replay_checked(&sg).map(|s| (s, sg.commands))) {
            Some(Ok(((sim, os, warn), cmds))) => {
                self.de = linuxde::LinuxDe::new();
                self.de.os_case = sim.case.id.clone();
                self.de.os = Some(os);
                self.term.lines = format!("Sessão restaurada por replay de {} comando(s). Cadeia de auditoria íntegra.\n", cmds.len());
                for w in &warn {
                    self.term.lines.push_str(&format!("ATENÇÃO: {}\n", w));
                }
                if let Some(w) = warn.first() {
                    self.toast(w);
                }
                self.phone.close();
                self.tablet.bind_case(&sim.case.id, false);
                self.sim = Some(sim);
                self.cmdlog = cmds;
                self.screen = Screen::Game;
                self.area = Area::Hardware;
                self.desk = DesktopState::new();
                if self.sim.as_ref().unwrap().report.submitted {
                    self.desk.eval_text = pericia_core::assess::render(self.sim.as_ref().unwrap(), &pericia_core::assess::evaluate(self.sim.as_ref().unwrap()));
                }
            }
            Some(Err(e)) => self.menu_msg = format!("Não foi possível restaurar: {}", e),
            None => self.menu_msg = "Nenhum caso salvo.".into(),
        }
    }
}

// ====================================================================== terminal

pub fn draw_terminal(app: &mut App, r: Rect, big: bool) {
    let s = app.ui.scale;
    let ih = 28.0 * s;
    let out_r = rect(r.x, r.y, r.w, r.h - ih - 4.0 * s);
    let text = app.term.lines.clone();
    let mut sc = app.term.scroll;
    app.ui.scroll_text(out_r, &text, &mut sc, if big { 12.5 } else { 11.5 }, F::Mono, T.term_fg, true);
    app.term.scroll = sc;
    let in_r = rect(r.x, r.y + r.h - ih, r.w, ih);
    let focused = app.ui.focus.as_deref() == Some("term");
    if focused && app.ui.key_up && !app.term.history.is_empty() {
        app.term.hist_idx = app.term.hist_idx.saturating_sub(1);
        app.term.input = app.term.history[app.term.hist_idx].clone();
    }
    if focused && app.ui.key_down && !app.term.history.is_empty() {
        app.term.hist_idx = (app.term.hist_idx + 1).min(app.term.history.len());
        app.term.input = app.term.history.get(app.term.hist_idx).cloned().unwrap_or_default();
    }
    let mut buf = std::mem::take(&mut app.term.input);
    let enter = app.ui.input("term", in_r, &mut buf, "digite um comando (ajuda) e pressione Enter");
    app.term.input = buf;
    if enter {
        let c = app.term.input.trim().to_string();
        app.term.input.clear();
        if c == "limpar" || c == "clear" {
            app.term.lines.clear();
        } else if !c.is_empty() {
            app.term.history.push(c.clone());
            app.term.hist_idx = app.term.history.len();
            let out = app.run(&c);
            if c.starts_with("finalizar") || c.starts_with("avaliar") {
                app.finish_case(out);
            }
        }
    }
}

// ====================================================================== menu

fn draw_menu(app: &mut App) {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.scale;
    clear_background(T.bg);
    // faixa de título
    app.ui.text("PERÍCIA SIM", 48.0 * s, 70.0 * s, 34.0, F::Bold, T.text);
    app.ui.text("Simulador de treinamento em informática forense — POPs SENASP vol. 5 (2024)", 50.0 * s, 98.0 * s, 14.0, F::Reg, T.dim);
    app.ui.text("Todo o conteúdo é fictício. A \"internet\" do jogo é emulada e nunca acessa a rede real.", 50.0 * s, 120.0 * s, 12.0, F::Reg, T.dim);
    let col1 = rect(48.0 * s, 150.0 * s, w * 0.52, h - 200.0 * s);
    app.ui.frame(col1, T.panel, T.border);
    let mut y = col1.y + 26.0 * s;
    app.ui.text("1. Módulo (POP)", col1.x + 16.0 * s, y, 14.0, F::Bold, T.text);
    y += 12.0 * s;
    for m in ModuleId::ALL {
        let r = rect(col1.x + 16.0 * s, y, col1.w - 32.0 * s, 34.0 * s);
        let active = app.menu_module == m;
        if app.ui.tab(r, &format!("POP {} — {}", m.code(), m.short()), active) {
            app.menu_module = m;
        }
        y += 38.0 * s;
    }
    y += 22.0 * s;
    app.ui.text("2. Dificuldade", col1.x + 16.0 * s, y, 14.0, F::Bold, T.text);
    y += 12.0 * s;
    let labels = ["1 · Guiado", "2 · Assistido", "3 · Autônomo", "4 · Avançado"];
    let bw = (col1.w - 32.0 * s - 18.0 * s) / 4.0;
    for (i, l) in labels.iter().enumerate() {
        let r = rect(col1.x + 16.0 * s + i as f32 * (bw + 6.0 * s), y, bw, 34.0 * s);
        if app.ui.tab(r, l, app.menu_diff == i as u8 + 1) {
            app.menu_diff = i as u8 + 1;
        }
    }
    y += 46.0 * s;
    let desc = match app.menu_diff {
        1 => "Mentor mostra o checklist e o próximo passo. Evidências diretas.",
        2 => "Mentor sugere o próximo passo sob demanda. Arquivos apagados, compostos, divergências.",
        3 => "Pouca assistência. Criptografia, fuso horário, CGNAT, mídias ocultas.",
        _ => "Sem orientação. Evidências distribuídas, eventos adversos e limitações severas.",
    };
    app.ui.text(desc, col1.x + 16.0 * s, y, 12.0, F::Reg, T.dim);
    y += 26.0 * s;
    app.ui.text("3. Semente ou ID do caso (opcional — reproduz um caso)", col1.x + 16.0 * s, y, 14.0, F::Bold, T.text);
    y += 10.0 * s;
    let mut seed = std::mem::take(&mut app.menu_seed);
    let enter = app.ui.input("seed", rect(col1.x + 16.0 * s, y, col1.w - 32.0 * s, 32.0 * s), &mut seed, "vazio = aleatório · ex.: 1234 ou C504-D3-000004D2");
    app.menu_seed = seed;
    y += 46.0 * s;
    let start = app.ui.button_c(rect(col1.x + 16.0 * s, y, 220.0 * s, 40.0 * s), "Iniciar novo caso", Color::new(0.16, 0.45, 0.28, 1.0), true) || enter;
    let has_save = save::has_current();
    let cont = app.ui.button_c(rect(col1.x + 246.0 * s, y, 220.0 * s, 40.0 * s), "Continuar caso salvo", T.panel2, has_save);
    if start {
        let txt = app.menu_seed.trim().to_string();
        if let Some((m, d, sd)) = parse_case_id(&txt) {
            app.start_case(m, d, sd);
        } else {
            let seed = if txt.is_empty() {
                (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(1) % 0xFFFF_FFFF) as u32
            } else {
                txt.parse::<u32>().unwrap_or_else(|_| pericia_core::rng::fnv1a(&txt))
            };
            let (m, d) = (app.menu_module, app.menu_diff);
            app.start_case(m, d, seed);
        }
        return;
    }
    if cont {
        app.resume();
        return;
    }
    if !app.menu_msg.is_empty() {
        app.ui.text(&app.menu_msg.clone(), col1.x + 16.0 * s, y + 62.0 * s, 12.0, F::Reg, T.warn);
    }
    // coluna 2: histórico e configurações
    let col2 = rect(col1.x + col1.w + 16.0 * s, col1.y, w - col1.w - 112.0 * s, col1.h);
    app.ui.frame(col2, T.panel, T.border);
    let mut y = col2.y + 26.0 * s;
    app.ui.text("Casos concluídos", col2.x + 16.0 * s, y, 14.0, F::Bold, T.text);
    y += 16.0 * s;
    if app.history.is_empty() {
        app.ui.text("Nenhum caso concluído ainda.", col2.x + 16.0 * s, y + 10.0 * s, 12.0, F::Reg, T.dim);
        y += 26.0 * s;
    }
    for e in app.history.iter().rev().take(8).cloned().collect::<Vec<_>>() {
        let col = if e.verdict.starts_with("Aprovado") { T.ok } else { T.bad };
        app.ui.text(&format!("{}  {:>3}/{}  {}", e.id, e.score, e.max, e.verdict), col2.x + 16.0 * s, y + 12.0 * s, 11.5, F::Mono, col);
        let r = rect(col2.x + col2.w - 96.0 * s, y - 2.0 * s, 80.0 * s, 20.0 * s);
        if app.ui.button(r, "Rever") {
            app.menu_seed = e.id.clone();
        }
        y += 24.0 * s;
    }
    y += 20.0 * s;
    app.ui.text("Configurações", col2.x + 16.0 * s, y, 14.0, F::Bold, T.text);
    y += 14.0 * s;
    let st = &app.settings;
    let resumo = format!(
        "Som {} · cena {} · sombras {} · escala {:.0}%",
        if st.muted { "silenciado".to_string() } else { format!("{:.0}%", st.vol_master * 100.0) },
        if st.render3d { "3D" } else { "2D" },
        if st.shadows { "ativadas" } else { "desativadas" },
        st.ui_scale * 100.0
    );
    app.ui.text(&resumo, col2.x + 16.0 * s, y + 18.0 * s, 11.5, F::Reg, T.dim);
    y += 28.0 * s;
    if app.ui.button(rect(col2.x + 16.0 * s, y, col2.w - 32.0 * s, 32.0 * s), "⚙  Abrir configurações (áudio, gráficos, controles, interface)") {
        config::open(app);
    }
    y += 40.0 * s;
    let howto = "Como jogar\n• Área de Hardware: gire a câmera com o botão direito, aproxime com a roda e clique nos equipamentos. Cada ação física vira um comando no terminal.\n• Estação Pericial: clique no PC ao fundo ou na aba superior. No PeritOS você examina as imagens, navega na rede emulada e redige o documento.\n• Encontrar a expressão-alvo não basta: procedimento, preservação e fundamentação são avaliados.";
    let lines = app.ui.wrap(howto, 11.5, F::Reg, col2.w - 32.0 * s);
    for l in lines {
        if y > col2.y + col2.h - 14.0 * s {
            break;
        }
        app.ui.text(&l, col2.x + 16.0 * s, y, 11.5, F::Reg, T.dim);
        y += 17.0 * s;
    }
    app.ui.text(&format!("Dados salvos em: {}", save::data_dir().display()), 48.0 * s, h - 22.0 * s, 10.5, F::Reg, T.dim);
}

// ====================================================================== jogo

fn top_bar(app: &mut App) -> f32 {
    let s = app.ui.scale;
    let h = 40.0 * s;
    let w = screen_width();
    app.ui.panel(rect(0.0, 0.0, w, h), T.panel);
    draw_line(0.0, h, w, h, 1.0, T.border);
    let (id, time, module) = app
        .sim
        .as_ref()
        .map(|x| (x.case.id.clone(), pericia_core::util::fmt_dt(x.clock, pericia_core::util::BRT), x.case.module.short().to_string()))
        .unwrap_or_default();
    app.ui.text(&id, 12.0 * s, 17.0 * s, 13.0, F::Bold, T.text);
    app.ui.text(&format!("{} · {}", module, time), 12.0 * s, 33.0 * s, 10.5, F::Reg, T.dim);
    let tw = 190.0 * s;
    let tx = w / 2.0 - tw - 4.0 * s;
    if app.ui.tab(rect(tx, 5.0 * s, tw, h - 10.0 * s), "▣ Ambiente (1ª pessoa)", app.area == Area::Hardware) {
        app.area = Area::Hardware;
        desktop::sync_laudo(app);
    }
    if app.ui.tab(rect(w / 2.0 + 4.0 * s, 5.0 * s, tw, h - 10.0 * s), "▭ Estação Pericial", app.area == Area::Station) {
        app.area = Area::Station;
    }
    let mut x = w - 8.0 * s;
    for (label, act) in [("Menu", 0), ("Terminal", 1), ("⚙ Configurações", 2)] {
        let bw = app.ui.measure(label, 13.0, F::Reg) + 24.0 * s;
        x -= bw + 4.0 * s;
        if app.ui.button(rect(x, 6.0 * s, bw, h - 12.0 * s), label) {
            match act {
                0 => {
                    desktop::sync_laudo(app);
                    app.screen = Screen::Menu;
                    app.history = save::history();
                }
                1 => app.term_open = !app.term_open,
                _ => config::open(app),
            }
        }
    }
    h
}

/// Ações físicas disponíveis para o item selecionado — cada uma é um comando do terminal.
fn item_actions(sim: &Sim, it: &Item) -> Vec<(String, String, bool)> {
    let id = it.id.to_lowercase();
    let mut v: Vec<(String, String, bool)> = vec![];
    let add = |v: &mut Vec<(String, String, bool)>, l: &str, c: String, en: bool| v.push((l.to_string(), c, en));
    add(&mut v, "Fotografar", format!("fotografar {}", id), true);
    if it.seal.is_some() || it.doc_seal.is_some() {
        add(&mut v, "Conferir lacre", format!("lacre conferir {}", id), true);
        add(&mut v, "Deslacrar", format!("deslacrar {}", id), it.seal.as_ref().map(|s| s.intact).unwrap_or(false));
    }
    if it.kind.is_computer() {
        if let Some(live) = &it.live {
            if it.power_on {
                add(&mut v, "Isolar da rede", format!("rede isolar {}", id), live.network_connected);
                add(&mut v, "Dump de memória", format!("memoria {}", id), live.memory_dump.is_none());
                add(&mut v, "Copiar volume montado", format!("volume copiar {}", id), live.mounted_encrypted.is_some());
            }
        }
        if it.power_on {
            add(&mut v, "Desligar (energia)", format!("energia desligar {}", id), true);
        }
        add(&mut v, "Abrir gabinete", format!("abrir {}", id), !it.power_on && !it.flag("aberto"));
        add(&mut v, "Inspecionar", format!("inspecionar {}", id), true);
        for c in &it.components {
            if c.kind != "tampa" && !c.removed && it.flag("aberto") {
                add(&mut v, &format!("Remover {}", c.label.to_lowercase()), format!("componente remover {} {}", id, c.id), true);
            }
        }
        for sl in &it.slots {
            if let (Some(m), None) = (&sl.media_id, &sl.hidden_by) {
                if it.flag("aberto") || it.kind == ItemKind::Notebook {
                    add(&mut v, &format!("Remover mídia {} ({})", m, sl.id), format!("midia remover {} {}", id, sl.id.to_lowercase()), !it.power_on);
                }
            }
        }
    }
    if let Some(ph) = &it.phone {
        add(&mut v, "▣ Ver a tela do aparelho", "@tela".into(), true);
        add(&mut v, "Isolar: Faraday bag", format!("isolar {} faraday", id), !ph.rf.isolates());
        add(&mut v, "Isolar: alumínio (3 camadas)", format!("isolar {} aluminio", id), !ph.rf.isolates());
        add(&mut v, "Modo avião", format!("airplane {}", id), !ph.airplane);
        add(&mut v, "Bateria externa", format!("carregar {}", id), !ph.charging);
        add(&mut v, "Desabilitar alarmes", format!("alarmes {}", id), !ph.alarms_disabled);
        for sim_c in &ph.sims {
            if sim_c.place == SimWhere::Slot {
                add(&mut v, &format!("Remover SIM (slot {})", sim_c.slot), format!("sim remover {} {}", id, sim_c.slot), true);
            }
        }
        if sim.case.module == ModuleId::M505 {
            add(&mut v, "Solicitar senhas", format!("solicitar-senhas {}", id), !it.flag("senhas_solicitadas"));
            add(&mut v, "Etiqueta na câmera frontal", format!("camera cobrir {}", id), !ph.front_camera_covered);
        }
        add(&mut v, "Senhas no termo", "senha".into(), true);
        if sim.case.module != ModuleId::M505 {
            add(&mut v, "Extrair (automático)", format!("extrair {}", id), !ph.wiped);
            add(&mut v, "Extrair: sistema de arquivos", format!("extrair {} fs", id), !ph.wiped);
            add(&mut v, "Extrair: lógica", format!("extrair {} logica", id), !ph.wiped);
        }
    }
    // mídias do item que podem ser adquiridas
    for mid in &it.media {
        let Some(m) = sim.case.media_by_id(mid) else { continue };
        if m.kind == MediaKind::FlashMovel {
            continue;
        }
        let reachable = it.kind.is_media() || it.slots.iter().any(|s| s.media_id.as_deref() == Some(mid) && s.hidden_by.is_none());
        if !reachable {
            continue;
        }
        add(&mut v, &format!("Bloqueador → {}", mid), format!("bloqueio {}", mid.to_lowercase()), true);
        add(&mut v, &format!("Adquirir imagem {}", mid), format!("imagem {}", mid.to_lowercase()), !sim.imaged.contains(mid));
        add(&mut v, &format!("Verificar hash {}", mid), format!("verificar-hash {}", mid.to_lowercase()), true);
    }
    add(&mut v, "Embalar", format!("embalar {}", id), !it.flag("embalado"));
    add(&mut v, "Apor lacre", format!("lacre aplicar {}", id), true);
    add(&mut v, "Etiquetar", format!("etiquetar {} {}", id, it.label), !it.flag("etiqueta"));
    v
}

fn hardware(app: &mut App, area: Rect) {
    if app.config.open {
        // configurações abertas: a cena segue desenhada, sem receber entrada nem teclas
        hardware_inner(app, area, true);
        return;
    }
    let typing = matches!(app.ui.focus.as_deref(), Some("term") | Some("pin") | Some("seed"));
    if tablet::keys(app, typing) {
        // tablet em mãos: a cena segue desenhada (sem o tablet 3D), sem receber entrada
        let clicked = app.ui.clicked && !app.ui.click_consumed;
        let wheel = app.ui.wheel;
        app.ui.click_consumed = true;
        app.ui.wheel = 0.0;
        hardware_inner(app, area, true);
        app.ui.click_consumed = !clicked;
        app.ui.wheel = wheel;
        app.ui.tooltip = None;
        app.hover_label = None;
        tablet::draw(app, area);
        return;
    }
    if app.phone.open.is_none() {
        hardware_inner(app, area, false);
        return;
    }
    // aparelho em mãos: a cena continua desenhada, mas não recebe entrada
    let clicked = app.ui.clicked && !app.ui.click_consumed;
    let wheel = app.ui.wheel;
    app.ui.click_consumed = true;
    app.ui.wheel = 0.0;
    hardware_inner(app, area, true);
    app.ui.click_consumed = !clicked;
    app.ui.wheel = wheel;
    app.ui.tooltip = None;
    app.hover_label = None;
    phone::draw(app, area);
}

fn hardware_inner(app: &mut App, area: Rect, modal: bool) {
    let s = app.ui.scale;
    let show_panel = app.selected.is_some() || app.show_help_overlay;
    let panel_w = if show_panel { 370.0 * s } else { 0.0 };
    let view = rect(area.x, area.y, area.w - panel_w, area.h);
    let typing = modal || matches!(app.ui.focus.as_deref(), Some("term") | Some("pin") | Some("seed") | Some("tnote"));
    if !app.settings.render3d {
        // modo 2D leve: cartões clicáveis
        app.ui.panel(view, T.bg);
        if let Some(sim) = &app.sim {
            let items: Vec<(String, String)> = sim.case.items.iter().filter(|i| !i.hidden || i.flag("descoberto")).map(|i| (i.id.clone(), format!("{} — {}", i.id, i.label))).collect();
            for (k, (id, label)) in items.iter().enumerate() {
                let r = rect(view.x + 20.0 * s, view.y + 20.0 * s + k as f32 * 58.0 * s, view.w - 40.0 * s, 50.0 * s);
                if app.ui.tab(r, label, app.selected.as_deref() == Some(id.as_str())) {
                    app.selected = Some(id.clone());
                }
            }
            let r = rect(view.x + 20.0 * s, view.y + view.h - 70.0 * s, 300.0 * s, 44.0 * s);
            if app.ui.button(r, "Ir para a Estação Pericial") {
                app.area = Area::Station;
                app.station_zoom = true;
            }
        }
        side_panel(app, rect(area.x + view.w, area.y, area.w - view.w, area.h));
        return;
    }
    if app.renderer.is_none() {
        app.renderer = Some(render::Renderer::new());
    }
    let Some(sim) = app.sim.as_ref() else { return };
    let case_id = sim.case.id.clone();
    let mut r = app.renderer.take().unwrap();
    r.screen_tex = app.de.snapshot.clone();
    let cam = app.player.camera(view);
    let mut c = app.player.camera(view);
    c.viewport = Some((view.x as i32, (screen_height() - view.y - view.h) as i32, view.w as i32, view.h as i32));
    let wrld = world::build(&mut r, sim, app.selected.as_deref(), app.hover_pick.as_deref());
    let lights = render::Lights {
        cam: cam.position,
        l: wrld.lights,
        sun_dir: vec3(0.9, -0.35, 0.12),
        sun: vec3(0.22, 0.24, 0.27),
        sky: vec3(0.42, 0.44, 0.48),
        ground: vec3(0.17, 0.16, 0.15),
        fog: vec3(0.55, 0.57, 0.60),
        fog_den: 0.018,
    };
    if !app.tablet.open && app.phone.open.is_none() && app.settings.held_tablet {
        held_tablet(&mut r, &cam);
    }
    // ouvinte e posições dos objetos para o som 3D
    let right = (cam.target - cam.position).cross(vec3(0.0, 1.0, 0.0)).normalize_or_zero();
    app.ear = (cam.position, right);
    app.item_pos.clear();
    for p in &wrld.picks {
        app.item_pos.insert(p.id.clone(), p.center);
    }
    r.shadows = app.settings.shadows;
    r.flush(&lights, &c);
    set_default_camera();
    app.renderer = Some(r);
    app.seat = wrld.station_seat;
    if app.player.case != case_id {
        app.player.case = case_id;
        app.player.place(wrld.start.0, wrld.start.1);
    }
    app.player.sens = app.settings.mouse_sens;
    app.player.invert_y = app.settings.invert_y;
    app.player.fov = app.settings.fov_deg.to_radians();
    if app.player.update(&wrld, !typing, !modal && view.contains(app.ui.mouse)) {
        app.ui.any_input = true;
    }
    if let Some(run) = app.player.step.take() {
        sfx::footstep(app, run);
    }
    if !typing && is_key_pressed(KeyCode::Tab) {
        let c = !app.player.captured;
        app.player.set_captured(c);
    }
    if app.player.captured && (app.ui.key_esc || modal) {
        app.player.set_captured(false);
    }
    // mira e seleção (alcance de braço: é preciso se aproximar)
    let aim = if app.player.captured { view.center() } else { app.ui.mouse };
    let (o, d) = world::ray(&cam, view, aim);
    let mut best: Option<(f32, usize)> = None;
    if view.contains(aim) {
        for (i, p) in wrld.picks.iter().enumerate() {
            if let Some(t) = world::ray_aabb(o, d, p.center, p.half) {
                if best.map(|b| t < b.0).unwrap_or(true) {
                    best = Some((t, i));
                }
            }
        }
    }
    const REACH: f32 = 2.6;
    app.hover_pick = best.filter(|(t, _)| *t <= REACH).map(|(_, i)| wrld.picks[i].id.clone());
    // rótulos flutuantes dos itens próximos
    for p in &wrld.picks {
        if !p.id.starts_with("ITEM") {
            continue;
        }
        let dist = (p.center - cam.position).length();
        let focus = app.selected.as_deref() == Some(p.id.as_str()) || app.hover_pick.as_deref() == Some(p.id.as_str());
        // tela limpa: só o objeto na mira ou o selecionado ganha rótulo
        if !focus || modal || dist > 6.0 {
            continue;
        }
        if let Some(sp) = world::project(&cam, view, p.center + vec3(0.0, p.half.y + 0.04, 0.0)) {
            if !view.contains(sp) {
                continue;
            }
            let sel = app.selected.as_deref() == Some(p.id.as_str());
            let tw = app.ui.measure(&p.id, 11.0, F::Bold) + 10.0 * s;
            let rr = rect(sp.x - tw / 2.0, sp.y - 18.0 * s, tw, 18.0 * s);
            app.ui.frame(rr, if sel { T.accent2 } else { Color::new(0.0, 0.0, 0.0, 0.55) }, if sel { T.highlight } else { T.border });
            app.ui.text(&p.id, rr.x + 5.0 * s, rr.y + 13.0 * s, 11.0, F::Bold, T.text);
        }
    }
    // vinheta suave nas bordas (sensação de lente)
    let vsteps = 10;
    for k in 0..vsteps {
        let t = k as f32 / vsteps as f32;
        let a = (1.0 - t) * 0.028;
        let inset = t * 90.0 * s;
        let c = Color::new(0.0, 0.0, 0.0, a);
        draw_rectangle(view.x, view.y + inset, view.w, 9.0 * s, c);
        draw_rectangle(view.x, view.y + view.h - inset - 9.0 * s, view.w, 9.0 * s, c);
        draw_rectangle(view.x + inset, view.y, 9.0 * s, view.h, c);
        draw_rectangle(view.x + view.w - inset - 9.0 * s, view.y, 9.0 * s, view.h, c);
    }
    // mira
    let cc = view.center();
    let col = if app.hover_pick.is_some() { T.highlight } else { Color::new(1.0, 1.0, 1.0, 0.7) };
    if app.player.captured {
        draw_line(cc.x - 7.0 * s, cc.y, cc.x - 2.0 * s, cc.y, 2.0, col);
        draw_line(cc.x + 2.0 * s, cc.y, cc.x + 7.0 * s, cc.y, 2.0, col);
        draw_line(cc.x, cc.y - 7.0 * s, cc.x, cc.y - 2.0 * s, 2.0, col);
        draw_line(cc.x, cc.y + 2.0 * s, cc.x, cc.y + 7.0 * s, 2.0, col);
    }
    if let Some((t, i)) = best.filter(|_| !modal) {
        let p = &wrld.picks[i];
        let near = t <= REACH;
        let txt = if near { format!("[clique ou E]  {}", p.label) } else { format!("{}  — aproxime-se", p.label) };
        let tw = app.ui.measure(&txt, 12.5, F::Reg) + 18.0 * s;
        let at = if app.player.captured { vec2(cc.x - tw / 2.0, cc.y + 22.0 * s) } else { vec2(aim.x + 16.0 * s, aim.y + 10.0 * s) };
        let rr = rect(at.x, at.y, tw, 24.0 * s);
        app.ui.frame(rr, Color::new(0.0, 0.0, 0.0, 0.75), if near { T.highlight } else { T.border });
        app.ui.text(&txt, rr.x + 9.0 * s, rr.y + 16.5 * s, 12.5, F::Reg, if near { T.text } else { T.dim });
    }
    let clicked = app.ui.click_in(view);
    let pressed_e = !typing && is_key_pressed(KeyCode::E);
    if clicked || pressed_e {
        match best.filter(|(t, _)| *t <= REACH).map(|(_, i)| wrld.picks[i].clone()) {
            Some(p) if p.id == "STATION" => {
                app.audio.play("cadeira", audio::Cat::Efeitos, 0.9);
                app.player.set_captured(false);
                app.area = Area::Station;
                app.station_zoom = true;
                app.toast("Você se sentou na estação pericial.");
            }
            Some(p) if p.id.starts_with("ITEM") => {
                sfx::interact(app, &p.id, p.center);
                app.selected = Some(p.id.clone());
                app.panel_scroll = 0.0;
                if app.player.captured {
                    app.player.set_captured(false);
                }
            }
            Some(p) => {
                let info = match p.id.as_str() {
                    "TOOL-WB" => "Bloqueador de escrita: impede que a leitura altere a mídia original (POP 5.01). Use 'Bloqueador → M-xx' no painel do item.",
                    "TOOL-CAM" => "Câmera: fixação fotográfica antes de qualquer intervenção ('Fotografar').",
                    _ => "Embalagens antiestáticas, Faraday bag, papel-alumínio e fita de lacre numerada.",
                };
                sfx::interact(app, &p.id, p.center);
                app.toast(info);
            }
            None if clicked && !app.player.captured => {
                app.selected = None;
            }
            None => {}
        }
    }
    // dica discreta (controles completos: tablet → Apoio → Controles)
    if !modal && app.settings.show_hints {
        hint_chips(app, view);
    }
    if show_panel {
        side_panel(app, rect(area.x + view.w, area.y, panel_w, area.h));
    }
}

/// Dicas discretas no canto inferior esquerdo; "T Tablet" também é clicável.
fn hint_chips(app: &mut App, view: Rect) {
    let s = app.ui.scale;
    let look = if app.player.captured { ("Tab", "soltar o mouse") } else { ("Btn. dir.", "olhar") };
    let chips = [("T", "Tablet"), ("E", "Interagir"), look];
    let mut x = view.x + 12.0 * s;
    let y = view.y + view.h - 30.0 * s;
    for (k, (key, what)) in chips.iter().enumerate() {
        let kw = app.ui.measure(key, 10.5, F::Bold) + 10.0 * s;
        let ww = app.ui.measure(what, 10.5, F::Reg) + 10.0 * s;
        let r = rect(x, y, kw + ww, 20.0 * s);
        let hov = k == 0 && app.ui.hover(r);
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.0, 0.0, if hov { 0.6 } else { 0.32 }));
        draw_rectangle_lines(r.x, r.y, kw, r.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.35));
        app.ui.text(key, r.x + 5.0 * s, r.y + 14.5 * s, 10.5, F::Bold, Color::new(1.0, 1.0, 1.0, 0.85));
        app.ui.text(what, r.x + kw + 5.0 * s, r.y + 14.5 * s, 10.5, F::Reg, Color::new(1.0, 1.0, 1.0, 0.7));
        if k == 0 && app.ui.click_in(r) {
            tablet::open(app);
        }
        x += r.w + 6.0 * s;
    }
}

/// Tablet do perito em mãos (canto inferior direito da visão), preso à câmera.
fn held_tablet(r: &mut render::Renderer, cam: &Camera3D) {
    let f = (cam.target - cam.position).normalize();
    let rt = f.cross(vec3(0.0, 1.0, 0.0)).normalize_or_zero();
    if rt == Vec3::ZERO {
        return;
    }
    let up = rt.cross(f);
    // tela inclinada para o perito; segurado baixo e à direita
    let a: f32 = 1.0;
    let n = up * a.cos() - f * a.sin();
    let b = -f * a.cos() - up * a.sin();
    let pos = cam.position + f * 0.42 - up * 0.32 + rt * 0.31;
    let yaw = Mat3::from_axis_angle(n, 0.35); // gira levemente para o centro
    let (x, y, z) = (yaw * rt, n, yaw * b);
    let m = Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), pos.extend(1.0)) * Mat4::from_translation(vec3(0.0, -0.007, 0.0));
    let tela = render::emis(render::Tex::Screen, render::rgb(70, 110, 150));
    r.model_held("tablet_perito", m, &[("tela", tela)]);
}

/// Painel compacto de ações do objeto selecionado (o estado detalhado está no tablet).
fn side_panel(app: &mut App, panel: Rect) {
    let s = app.ui.scale;
    app.ui.frame(panel, T.panel, T.border);
    let Some(sim) = &app.sim else { return };
    let sel = app.selected.clone().and_then(|id| sim.case.item(&id).cloned());
    let pad = 10.0 * s;
    let mut y = panel.y + pad;
    let close_r = rect(panel.x + panel.w - pad - 26.0 * s, y, 26.0 * s, 24.0 * s);
    if app.ui.button(close_r, "✕") {
        app.selected = None;
        app.show_help_overlay = false;
        return;
    }
    match sel {
        None => {
            app.ui.text("Local de exame", panel.x + pad, y + 16.0 * s, 14.0, F::Bold, T.text);
            y += 34.0 * s;
            let local = sim.case.setting == Setting::Local;
            let msg = if local {
                "Documente o ambiente, reconheça e isole os equipamentos antes de intervir."
            } else {
                "Laboratório: o material chegou lacrado; confira a documentação antes do deslacre."
            };
            for l in app.ui.wrap(msg, 12.0, F::Reg, panel.w - pad * 2.0) {
                app.ui.text(&l, panel.x + pad, y, 12.0, F::Reg, T.dim);
                y += 17.0 * s;
            }
            y += 8.0 * s;
            if app.ui.button(rect(panel.x + pad, y, panel.w - pad * 2.0, 30.0 * s), "Inspecionar o local (busca minuciosa)") {
                app.run("inspecionar local");
            }
            y += 36.0 * s;
            if app.ui.button(rect(panel.x + pad, y, panel.w - pad * 2.0, 30.0 * s), "Missão e quesitos no tablet (T)") {
                app.tablet.tab = 0;
                tablet::open(app);
            }
        }
        Some(it) => {
            let title = phone::clip_text(&app.ui, &format!("{} — {}", it.id, it.label), 14.0, F::Bold, panel.w - pad * 2.0 - 32.0 * s);
            app.ui.text(&title, panel.x + pad, y + 16.0 * s, 14.0, F::Bold, T.text);
            y += 24.0 * s;
            // uma linha de estado — a ficha completa (sem spoilers) fica no tablet
            let mut st = vec![it.kind.label().to_string()];
            if let Some(sl) = &it.seal {
                st.push(format!("lacre {}", if sl.intact { "íntegro" } else { "rompido" }));
            }
            if it.kind.is_computer() {
                st.push(if it.power_on { "LIGADO".into() } else { "desligado".into() });
            }
            if let Some(ph) = &it.phone {
                st.push(format!("bateria {:.0}%", ph.battery));
            }
            app.ui.text(&st.join(" · "), panel.x + pad, y + 10.0 * s, 11.0, F::Reg, T.dim);
            y += 18.0 * s;
            if app.ui.button(rect(panel.x + pad, y, panel.w - pad * 2.0, 26.0 * s), "Estado e cuidados no tablet (T)") {
                app.tablet.tab = 1;
                app.tablet.item = Some(it.id.clone());
                tablet::open(app);
            }
            y += 34.0 * s;
            // campo para senha/PIN (desbloqueio) quando for móvel
            if let Some(ph) = &it.phone {
                if ph.boot != Boot::Desbloqueado && ph.boot != Boot::Desligado {
                    let r = rect(panel.x + pad, y, panel.w - pad * 2.0 - 110.0 * s, 28.0 * s);
                    let mut buf = std::mem::take(&mut app.panel_input);
                    let enter = app.ui.input("pin", r, &mut buf, "PIN / senha do aparelho");
                    app.panel_input = buf;
                    if (app.ui.button(rect(r.x + r.w + 6.0 * s, y, 104.0 * s, 28.0 * s), "Desbloquear") || enter) && !app.panel_input.is_empty() {
                        let c = format!("desbloquear {} {}", it.id.to_lowercase(), app.panel_input);
                        app.panel_input.clear();
                        app.run(&c);
                    }
                    y += 34.0 * s;
                }
            }
            app.ui.text("AÇÕES", panel.x + pad, y + 10.0 * s, 10.5, F::Bold, T.dim);
            y += 16.0 * s;
            let acts = item_actions(app.sim.as_ref().unwrap(), &it);
            let bw = (panel.w - pad * 2.0 - 6.0 * s) / 2.0;
            let bh = 28.0 * s;
            for (i, (l, c, en)) in acts.iter().enumerate() {
                let r = rect(panel.x + pad + (i % 2) as f32 * (bw + 6.0 * s), y + (i / 2) as f32 * (bh + 5.0 * s), bw, bh);
                if r.y + r.h > panel.y + panel.h - 4.0 * s {
                    break;
                }
                if app.ui.hover(r) {
                    app.ui.tooltip = Some(c.clone());
                }
                if app.ui.button_c(r, l, if c == "@tela" { T.accent2 } else { T.panel2 }, *en) {
                    if c == "@tela" {
                        app.phone.close();
                        app.phone.open = Some(it.id.clone());
                    } else {
                        app.run(c);
                    }
                }
            }
            if let Some(tt) = app.ui.tooltip.clone() {
                app.hover_label = Some(format!("comando: {}", tt));
            }
        }
    }
}

fn station(app: &mut App, area: Rect) {
    let s = app.ui.scale;
    app.station_zoom = true;
    linuxde::draw(app, area);
    app.de.preview_age += 1;
    if app.settings.render3d && app.de.preview_age >= 90 {
        app.de.preview_age = 0;
        linuxde::capture(app, area);
    }
    let r = rect(area.x + area.w - 170.0 * s, area.y + area.h - 30.0 * s, 164.0 * s, 24.0 * s);
    if app.ui.button(r, "◄ Levantar da estação") {
        app.audio.play("cadeira", audio::Cat::Efeitos, 0.8);
        linuxde::capture(app, area);
        desktop::sync_laudo(app);
        app.area = Area::Hardware;
        let (p, y) = app.seat;
        app.player.place(p, y);
    }
}

fn game(app: &mut App) {
    clear_background(T.bg);
    // Esc no ambiente: fecha o painel do objeto; sem nada aberto, abre as configurações
    if app.ui.key_esc && app.area == Area::Hardware && !app.config.open && !app.tablet.open && app.phone.open.is_none() && !app.player.captured && app.ui.focus.is_none() {
        app.ui.key_esc = false;
        if app.selected.is_some() || app.show_help_overlay {
            app.selected = None;
            app.show_help_overlay = false;
        } else {
            config::open(app);
        }
    }
    let top = top_bar(app);
    let s = app.ui.scale;
    // na área de trabalho Linux o terminal do simulador fica oculto (o Linux tem o seu)
    let show_term = app.term_open && !(app.area == Area::Station && (app.station_zoom || !app.settings.render3d));
    let term_h = if show_term { 190.0 * s } else { 0.0 };
    let area = rect(0.0, top + 1.0, screen_width(), screen_height() - top - 1.0 - term_h);
    match app.area {
        Area::Hardware => hardware(app, area),
        Area::Station => station(app, area),
    }
    if show_term {
        let tr = rect(4.0 * s, screen_height() - term_h + 2.0 * s, screen_width() - 8.0 * s, term_h - 6.0 * s);
        draw_terminal(app, tr, false);
    }
    if let Some((t, until)) = app.toast.clone() {
        if get_time() < until {
            let w = (app.ui.measure(&t, 12.5, F::Reg) + 24.0 * s).min(screen_width() - 40.0 * s);
            let r = rect((screen_width() - w) / 2.0, top + 12.0 * s, w, 30.0 * s);
            app.ui.frame(r, Color::new(0.05, 0.08, 0.12, 0.95), T.accent);
            app.ui.text(&t, r.x + 12.0 * s, r.y + 20.0 * s, 12.5, F::Reg, T.text);
            app.ui.any_input = true;
        } else {
            app.toast = None;
        }
    }
}

// ====================================================================== roteiro de teste (capturas automáticas)

struct Script {
    steps: Vec<String>,
    i: usize,
    wait: u32,
    dir: String,
    pending: Option<String>,
    /// Movimento suave do jogador: (origem, destino, yaw0, yaw1, pitch0, pitch1, total, feito).
    anim: Option<(Vec3, Vec3, f32, f32, f32, f32, u32, u32)>,
    /// Distância andada desde o último passo (som dos passos nas caminhadas do roteiro).
    walk_acc: f32,
}

/// Legenda de demonstração: faixa escura com o texto, na parte inferior central.
fn draw_caption(app: &mut App, text: &str) {
    let s = app.ui.scale;
    let (title, body) = text.split_once(" | ").unwrap_or((text, ""));
    let w = (app.ui.measure(title, 16.0, F::Bold).max(app.ui.measure(body, 13.0, F::Reg)) + 40.0 * s).min(screen_width() - 40.0 * s);
    let h = if body.is_empty() { 40.0 * s } else { 62.0 * s };
    let r = rect((screen_width() - w) / 2.0, screen_height() - h - 46.0 * s, w, h);
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.02, 0.03, 0.05, 0.86));
    draw_rectangle(r.x, r.y, 4.0 * s, r.h, T.highlight);
    app.ui.text(title, r.x + 18.0 * s, r.y + 26.0 * s, 16.0, F::Bold, T.highlight);
    if !body.is_empty() {
        app.ui.text(body, r.x + 18.0 * s, r.y + 50.0 * s, 13.0, F::Reg, T.text);
    }
}

fn run_script(app: &mut App, sc: &mut Script) -> bool {
    if let Some((p0, p1, y0, y1, a0, a1, n, k)) = sc.anim {
        let t = (k + 1) as f32 / n as f32;
        let e = t * t * (3.0 - 2.0 * t); // suavização
        let np = p0.lerp(p1, e);
        sc.walk_acc += (np - app.player.pos).length();
        if sc.walk_acc > 0.72 {
            sc.walk_acc -= 0.72;
            sfx::footstep(app, false);
        }
        app.player.pos = np;
        app.player.yaw = y0 + (y1 - y0) * e;
        app.player.pitch = a0 + (a1 - a0) * e;
        sc.anim = if k + 1 >= n { None } else { Some((p0, p1, y0, y1, a0, a1, n, k + 1)) };
        return true;
    }
    if sc.wait > 0 {
        sc.wait -= 1;
        return true;
    }
    let Some(step) = sc.steps.get(sc.i).cloned() else { return false };
    sc.i += 1;
    sc.wait = 3;
    let mut p = step.splitn(2, ' ');
    let k = p.next().unwrap_or("");
    let arg = p.next().unwrap_or("").to_string();
    match k {
        "new" => {
            if let Some((m, d, sd)) = parse_case_id(&arg) {
                app.start_case(m, d, sd);
            }
        }
        "cmd" => {
            let out = app.run(&arg);
            if arg.starts_with("finalizar") {
                app.finish_case(out);
            }
        }
        "area" => app.area = if arg == "station" { Area::Station } else { Area::Hardware },
        "zoom" => app.station_zoom = arg == "on",
        "select" => {
            app.selected = Some(arg.to_uppercase());
        }
        "pos" => {
            let v: Vec<f32> = arg.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            if v.len() >= 2 {
                app.player.pos = vec3(v[0], 0.0, v[1]);
            }
            if v.len() >= 3 {
                app.player.yaw = v[2];
            }
            if v.len() >= 4 {
                app.player.pitch = v[3];
            }
            app.player.case = app.sim.as_ref().map(|s| s.case.id.clone()).unwrap_or_default();
        }
        "walk" | "look" => {
            // walk x z yaw pitch quadros | look yaw pitch quadros
            let v: Vec<f32> = arg.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            let (to, yaw, pitch, n) = if k == "walk" && v.len() >= 5 {
                (vec3(v[0], 0.0, v[1]), v[2], v[3], v[4] as u32)
            } else if v.len() >= 3 {
                (app.player.pos, v[0], v[1], v[2] as u32)
            } else {
                return true;
            };
            // menor giro angular
            let mut y1 = yaw;
            let y0 = app.player.yaw;
            while y1 - y0 > std::f32::consts::PI {
                y1 -= std::f32::consts::TAU;
            }
            while y1 - y0 < -std::f32::consts::PI {
                y1 += std::f32::consts::TAU;
            }
            app.player.case = app.sim.as_ref().map(|s| s.case.id.clone()).unwrap_or_default();
            sc.anim = Some((app.player.pos, to, y0, y1, app.player.pitch, pitch, n.max(1), 0));
            sc.wait = 0;
        }
        "caption" => app.caption = if arg.is_empty() || arg == "off" { None } else { Some(arg) },
        "panel" => app.show_help_overlay = arg == "on",
        "cohere" => {
            desktop::sync_laudo(app);
            let c = pericia_core::report::check_coherence(app.sim.as_ref().unwrap());
            app.desk.laudo_msg = if c.problems.is_empty() { "Nenhuma incoerência detectada entre o texto e a auditoria.".into() } else { format!("ATENÇÃO:\n- {}", c.problems.join("\n- ")) };
        }
        "export" => {
            app.desk.laudo_msg = match save::export(app.sim.as_ref().unwrap()) {
                Ok((p, md)) => format!("Exportado:\n{}\n{}", p.display(), md.display()),
                Err(e) => format!("ERRO ao exportar: {}", e),
            };
        }
        "deselect" => app.selected = None,
        "yaw" => app.player.yaw = arg.parse().unwrap_or(0.0),
        "pitch" => app.player.pitch = arg.parse().unwrap_or(0.0),
        "app" => {
            let a = DApp::ALL.iter().copied().find(|x| x.name().to_lowercase().starts_with(&arg.to_lowercase())).unwrap_or(DApp::Caso);
            desktop::switch(app, a);
        }
        "url" => app.desk.nav_url = arg,
        "navdomain" => {
            let d = app.sim.as_ref().and_then(|s| s.case.net.as_ref().map(|n| n.domains[0].name.clone())).unwrap_or_default();
            app.desk.nav_url = format!("https://{}/{}", d, arg);
            app.desk.nav_tool = d;
        }
        "evsel" => {
            let mut it = arg.splitn(2, ' ');
            app.desk.ev_sel = Some((it.next().unwrap_or("").into(), it.next().unwrap_or("").into()));
        }
        "evout" => app.desk.ev_out = app.run(&arg),
        "navgo" => {
            let out = app.run(&format!("navegar {}", app.desk.nav_url.clone()));
            app.desk.nav_links = out.lines().find(|l| l.starts_with("Links:")).map(|l| l.trim_start_matches("Links:").split_whitespace().map(|x| x.to_string()).collect()).unwrap_or_default();
            app.desk.nav_view = out;
        }
        "laudo" => desktop::load_laudo(app),
        "login" => {
            linuxde::ensure_os(app);
            app.de.logged = true;
            if app.de.terms.is_empty() {
                linuxde::open(app, linuxde::WinKind::Terminal(0));
            }
        }
        "lt" => {
            if app.de.terms.is_empty() {
                linuxde::open(app, linuxde::WinKind::Terminal(0));
            }
            linuxde::exec_line(app, 0, &arg);
        }
        "deopen" => {
            let k = match arg.as_str() {
                "files" => linuxde::WinKind::Files,
                "editor" => linuxde::WinKind::Editor,
                "nav" => linuxde::WinKind::Navegador,
                "doc" => linuxde::WinKind::Documento,
                "caso" => linuxde::WinKind::Caso,
                "linha" => linuxde::WinKind::Linha,
                "aval" => linuxde::WinKind::Avaliacao,
                "evid" => linuxde::WinKind::Evidencias,
                "custodia" => linuxde::WinKind::Custodia,
                "manual" => linuxde::WinKind::Manual,
                _ => linuxde::WinKind::Terminal(0),
            };
            app.de.menu_open = false;
            linuxde::open(app, k);
        }
        "cwd" => app.de.files_cwd = arg,
        "edit" => linuxde::open_in_editor(app, &arg),
        "menuopen" => app.de.menu_open = true,
        "menuclose" => app.de.menu_open = false,
        "phone" => {
            app.phone.close();
            app.phone.open = if arg == "off" { None } else { Some(arg.to_uppercase()) };
        }
        "papp" => {
            let id = app.phone.open.clone().unwrap_or_default();
            let out = app.run(&format!("tela {} {}", id.to_lowercase(), arg));
            if !out.starts_with("ERRO") {
                app.phone.app = arg;
                app.phone.sub = None;
            }
        }
        "psub" => app.phone.sub = arg.parse().ok(),
        "ptab" => app.phone.tab = arg.parse().unwrap_or(0),
        "pemerg" => app.phone.emergency = true,
        // tablet: "tablet missao|objeto|apoio|notas|off [item|sub]", "tnote <tag>|<texto>", "tnedit <id>", "tndel <id>"
        "tablet" => {
            let mut a = arg.split_whitespace();
            match a.next().unwrap_or("") {
                "off" => tablet::close(app),
                t => {
                    app.tablet.tab = match t { "objeto" => 1, "apoio" => 2, "notas" => 3, _ => 0 };
                    if let Some(x) = a.next() {
                        if app.tablet.tab == 1 { app.tablet.item = Some(x.to_uppercase()); } else { app.tablet.sub = x.parse().unwrap_or(0); }
                    }
                    app.tablet.scroll = 0.0;
                    tablet::open(app);
                }
            }
        }
        "tscroll" => app.tablet.scroll = arg.parse().unwrap_or(0.0),
        "tnote" => {
            let (tag, txt) = arg.split_once('|').unwrap_or(("Observação", arg.as_str()));
            app.tablet.new_draft();
            app.tablet.draft_tag = tablet::TAGS.iter().position(|t| *t == tag.trim()).unwrap_or(0);
            app.tablet.draft = txt.replace("\\n", "\n");
            let clock = app.sim.as_ref().map(|x| x.clock).unwrap_or(0);
            app.tablet.save_draft(clock);
        }
        "tndraft" => {
            app.tablet.new_draft();
            app.tablet.draft = arg.replace("\\n", "\n");
        }
        "tnedit" => {
            let (id, txt) = arg.split_once(' ').unwrap_or((arg.as_str(), ""));
            app.tablet.load_note(id.parse().unwrap_or(0));
            if !txt.is_empty() {
                app.tablet.draft = txt.to_string();
                let clock = app.sim.as_ref().map(|x| x.clock).unwrap_or(0);
                app.tablet.save_draft(clock);
            }
        }
        "tndel" => app.tablet.delete(arg.parse().unwrap_or(0)),
        "resume" => app.resume(),
        "config" => match arg.as_str() {
            "off" => config::close(app),
            t => {
                app.config.tab = t.parse().unwrap_or(0);
                config::open(app);
            }
        },
        "press" => {
            // simula uma interação: "press <id do pick>" (item ou ferramenta) — som de tocar o objeto
            let id = arg.to_uppercase();
            if id == "STATION" {
                app.audio.play("cadeira", audio::Cat::Efeitos, 0.9);
                app.area = Area::Station;
                app.station_zoom = true;
            } else if let Some(p) = app.item_pos.get(&id).copied() {
                sfx::interact(app, &id, p);
                if id.starts_with("ITEM") {
                    app.selected = Some(id);
                }
            }
        }
        "menu" => {
            app.screen = Screen::Menu;
            app.history = save::history();
        }
        "term" => app.term_open = arg == "on",
        "shot" => {
            sc.pending = Some(arg);
            sc.wait = 2;
            return true;
        }
        "wait" => sc.wait = arg.parse().unwrap_or(10),
        _ => {}
    }
    true
}

#[macroquad::main(conf)]
async fn main() {
    let fonts = Fonts::load();
    let settings = save::load_settings();
    let vols = settings.volumes();
    let mut app = App {
        ui: Ui::new(fonts),
        screen: Screen::Menu,
        area: Area::Hardware,
        sim: None,
        cmdlog: vec![],
        term: Term { lines: String::new(), input: String::new(), scroll: 0.0, history: vec![], hist_idx: 0 },
        player: fps::Player::new(),
        renderer: None,
        hover_pick: None,
        seat: (vec3(0.0, 0.0, 0.0), std::f32::consts::PI),
        selected: None,
        hover_label: None,
        desk: DesktopState::new(),
        station_zoom: false,
        settings,
        menu_module: ModuleId::M501,
        menu_diff: 1,
        menu_seed: String::new(),
        menu_msg: String::new(),
        panel_scroll: 0.0,
        panel_input: String::new(),
        term_open: true,
        toast: None,
        history: save::history(),
        show_help_overlay: false,
        phone: phone::PhoneUi::default(),
        tablet: tablet::TabletUi::default(),
        audio: audio::Audio::new(vols),
        ear: (Vec3::ZERO, Vec3::X),
        item_pos: Default::default(),
        step_n: 0,
        key_n: 0,
        config: config::ConfigUi::default(),
        caption: None,
        de: linuxde::LinuxDe::new(),
    };
    // roteiro opcional para capturas automáticas (validação headless)
    let mut script = std::env::var("PERICIA_SCRIPT").ok().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| Script {
        steps: t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty() && !l.starts_with('#')).collect(),
        i: 0,
        wait: 5,
        dir: std::env::var("PERICIA_SHOTS").unwrap_or_else(|_| ".".into()),
        pending: None,
        anim: None,
        walk_acc: 0.0,
    });
    // gravação de vídeo de roteiro: limita a 30 quadros/s para que o tempo seja real
    let realtime = std::env::var("PERICIA_REALTIME").is_ok();
    let mut shot_n = 0;
    let mut idle_frames = 0u32;
    let mut perf_log: Vec<f32> = vec![];
    // vídeo de demonstração quadro a quadro (30 q/s), com o áudio mixado em sincronia
    let mut video = std::env::var("PERICIA_VIDEO").ok().and_then(|path| video::Pipe::open(&path, screen_width() as u32, screen_height() as u32));
    loop {
        let t0 = std::time::Instant::now();
        let scale = app.settings.ui_scale;
        app.ui.begin(scale);
        let held = if app.config.open { Some(app.ui.hold()) } else { None };
        match app.screen {
            Screen::Menu => draw_menu(&mut app),
            Screen::Game => game(&mut app),
        }
        if let Some(h) = held {
            app.ui.release(h);
            config::draw(&mut app);
        }
        app.audio.set_volumes(app.settings.volumes());
        sfx::update(&mut app);
        if let Some(c) = app.caption.clone() {
            draw_caption(&mut app, &c);
        }
        if let Some(v) = video.as_mut() {
            v.frame(&get_screen_data().bytes);
            app.audio.tick(1.0 / 30.0);
        }
        perf_log.push(t0.elapsed().as_secs_f32() * 1000.0);
        if realtime {
            let el = t0.elapsed().as_secs_f32();
            if el < 1.0 / 30.0 {
                std::thread::sleep(std::time::Duration::from_secs_f32(1.0 / 30.0 - el));
            }
        }
        if let Some(sc) = &mut script {
            if sc.wait == 0 {
                if let Some(name) = sc.pending.take() {
                    let n = if name.is_empty() { format!("{:02}", shot_n) } else { name };
                    let path = format!("{}/{}.png", sc.dir, n);
                    if let Some(dir) = std::path::Path::new(&path).parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    get_screen_data().export_png(&path);
                    shot_n += 1;
                }
            }
            if !run_script(&mut app, sc) {
                app.audio.finish();
                if let Some(v) = video.take() {
                    v.close();
                }
                let avg = perf_log.iter().sum::<f32>() / perf_log.len().max(1) as f32;
                let mut sorted = perf_log.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let p95 = sorted.get(sorted.len() * 95 / 100).copied().unwrap_or(0.0);
                println!("frames={} cpu_ms_avg={:.2} cpu_ms_p95={:.2}", perf_log.len(), avg, p95);
                break;
            }
        }
        // economia de energia: sem entrada por alguns quadros → reduz a taxa de redesenho
        if app.settings.low_power && script.is_none() {
            if app.ui.any_input {
                idle_frames = 0;
            } else {
                idle_frames += 1;
            }
            if idle_frames > 30 {
                std::thread::sleep(std::time::Duration::from_millis(45));
            }
        }
        next_frame().await;
    }
}
