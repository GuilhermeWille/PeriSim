//! Tablet do perito: central de informação aberta sobre a cena (tecla T).
//!
//! Só leitura do motor — abrir, navegar e fechar o tablet não executa comandos,
//! não avança o relógio e não toca a auditoria. As quatro áreas:
//!   Missão     objetivo, histórico, quesitos, autorizações e tarefas (conforme o nível)
//!   Objeto     ficha do objeto de análise sem spoilers (`pericia_core::tablet::objeto`)
//!   Apoio      mentor, resumo do POP, comandos e controles
//!   Anotações  bloco do perito: criar, ver, editar, excluir (arquivo próprio, fora do
//!              motor, da auditoria e da avaliação)

use crate::phone::{clip_text, rrect};
use crate::save;
use crate::ui::*;
use crate::App;
use macroquad::prelude::*;
use pericia_core::engine::Sim;
use pericia_core::tablet as tb;
use serde_json::{json, Value};

pub const TAGS: [&str; 5] = ["Observação", "Hipótese", "Horário", "Identificador", "Pendência"];

#[derive(Clone, Debug, PartialEq)]
pub struct Nota {
    pub id: u32,
    pub tag: String,
    pub texto: String,
    /// Instante simulado em que foi criada (só leitura do relógio do caso).
    pub criada: i64,
    pub editada: Option<i64>,
}

#[derive(Default)]
pub struct TabletUi {
    pub open: bool,
    pub tab: u8,
    pub sub: u8,
    pub item: Option<String>,
    pub scroll: f32,
    pub notas: Vec<Nota>,
    pub notas_case: String,
    /// Nota em edição (None = rascunho de nota nova).
    pub edit: Option<u32>,
    pub draft: String,
    pub draft_tag: usize,
    pub draft_scroll: f32,
    pub confirm_del: Option<(u32, f64)>,
    pub msg: Option<(String, f64)>,
    pub list_scroll: f32,
    /// Sons pendentes (gravar/excluir anotação), tocados pelo desenho de som.
    pub sfx: Vec<&'static str>,
}

// ---------------------------------------------------------------- persistência das anotações

fn notes_path() -> std::path::PathBuf {
    save::data_dir().join("saves").join("anotacoes.json")
}

pub fn notes_to_json(case_id: &str, notas: &[Nota]) -> String {
    let v = json!({
        "formato": 1,
        "case_id": case_id,
        "notas": notas.iter().map(|n| json!({
            "id": n.id, "tag": n.tag, "texto": n.texto, "criada": n.criada, "editada": n.editada,
        })).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&v).unwrap()
}

/// Lê as anotações de um JSON; devolve vazio se for de outro caso ou estiver ilegível.
pub fn notes_from_json(case_id: &str, s: &str) -> Vec<Nota> {
    let Ok(v) = serde_json::from_str::<Value>(s) else { return vec![] };
    if v["case_id"].as_str() != Some(case_id) {
        return vec![];
    }
    v["notas"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|n| {
                    Some(Nota {
                        id: n["id"].as_u64()? as u32,
                        tag: n["tag"].as_str().unwrap_or(TAGS[0]).to_string(),
                        texto: n["texto"].as_str()?.to_string(),
                        criada: n["criada"].as_i64().unwrap_or(0),
                        editada: n["editada"].as_i64(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

impl TabletUi {
    /// Carrega (ou zera) as anotações para o caso em curso.
    pub fn bind_case(&mut self, case_id: &str, fresh: bool) {
        self.notas = if fresh { vec![] } else { std::fs::read_to_string(notes_path()).map(|s| notes_from_json(case_id, &s)).unwrap_or_default() };
        self.notas_case = case_id.to_string();
        self.new_draft();
        self.item = None;
        self.open = false;
        if fresh {
            self.persist();
        }
    }
    fn persist(&mut self) {
        let dir = save::data_dir().join("saves");
        let _ = std::fs::create_dir_all(&dir);
        if let Err(e) = save::write_atomic(&notes_path(), notes_to_json(&self.notas_case, &self.notas).as_bytes()) {
            self.msg = Some((format!("ERRO ao gravar anotações: {}", e), get_time() + 4.0));
        }
    }
    pub fn save_draft(&mut self, clock: i64) {
        let texto = self.draft.trim_end().to_string();
        if texto.trim().is_empty() {
            self.msg = Some(("Anotação vazia não foi gravada.".into(), get_time() + 3.0));
            return;
        }
        let tag = TAGS[self.draft_tag.min(TAGS.len() - 1)].to_string();
        match self.edit.and_then(|id| self.notas.iter_mut().find(|n| n.id == id)) {
            Some(n) => {
                n.texto = texto;
                n.tag = tag;
                n.editada = Some(clock);
            }
            None => {
                let id = self.notas.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                self.notas.push(Nota { id, tag, texto, criada: clock, editada: None });
                self.edit = Some(id);
            }
        }
        self.persist();
        self.sfx.push("nota_gravada");
        self.msg = Some(("Anotação gravada.".into(), get_time() + 2.5));
    }
    pub fn delete(&mut self, id: u32) {
        self.notas.retain(|n| n.id != id);
        if self.edit == Some(id) {
            self.new_draft();
        }
        self.persist();
        self.sfx.push("papel");
        self.msg = Some(("Anotação excluída.".into(), get_time() + 2.5));
    }
    pub fn new_draft(&mut self) {
        self.edit = None;
        self.draft.clear();
        self.draft_tag = 0;
        self.draft_scroll = 0.0;
    }
    pub fn load_note(&mut self, id: u32) {
        if let Some(n) = self.notas.iter().find(|n| n.id == id) {
            self.edit = Some(id);
            self.draft = n.texto.clone();
            self.draft_tag = TAGS.iter().position(|t| *t == n.tag).unwrap_or(0);
            self.draft_scroll = 0.0;
        }
    }
}

// ---------------------------------------------------------------- texto formatado

enum B {
    H(String),
    P(String),
    Dim(String),
    Li(String),
    Chk(bool, String),
    Warn(String),
    Gap,
}

/// Desenha blocos com quebra de linha e rolagem vertical (em pixels).
fn rich(ui: &mut Ui, r: Rect, blocks: &[B], scroll: &mut f32) {
    let s = ui.scale;
    let pad = 12.0 * s;
    let maxw = r.w - pad * 2.0 - 10.0 * s;
    // medição
    let mut lines: Vec<(f32, String, f32, F, Color, f32)> = vec![]; // (x_off, texto, tamanho, fonte, cor, altura)
    for b in blocks {
        match b {
            B::H(t) => {
                lines.push((0.0, String::new(), 6.0, F::Reg, T.text, 6.0 * s));
                for l in ui.wrap(t, 13.5, F::Bold, maxw) {
                    lines.push((0.0, l, 13.5, F::Bold, Color::new(0.98, 0.62, 0.22, 1.0), 21.0 * s));
                }
            }
            B::P(t) | B::Dim(t) | B::Warn(t) => {
                let col = match b {
                    B::Dim(_) => T.dim,
                    B::Warn(_) => T.warn,
                    _ => T.text,
                };
                for l in ui.wrap(t, 12.0, F::Reg, maxw) {
                    lines.push((0.0, l, 12.0, F::Reg, col, 17.0 * s));
                }
            }
            B::Li(t) | B::Chk(_, t) => {
                let ind = 20.0 * s;
                let (mark, col) = match b {
                    B::Chk(true, _) => ("✔", T.ok),
                    B::Chk(false, _) => ("○", T.dim),
                    _ => ("•", T.dim),
                };
                for (k, l) in ui.wrap(t, 12.0, F::Reg, maxw - ind).into_iter().enumerate() {
                    if k == 0 {
                        lines.push((-1.0, mark.to_string(), 12.0, F::Bold, col, 0.0));
                    }
                    let c = if matches!(b, B::Chk(true, _)) { T.dim } else { T.text };
                    lines.push((ind, l, 12.0, F::Reg, c, 17.0 * s));
                }
            }
            B::Gap => lines.push((0.0, String::new(), 6.0, F::Reg, T.text, 8.0 * s)),
        }
    }
    let total: f32 = lines.iter().map(|l| l.5).sum();
    let maxs = (total - (r.h - pad * 2.0)).max(0.0);
    if ui.hover(r) && ui.wheel != 0.0 {
        *scroll -= ui.wheel.signum() * 40.0 * s;
    }
    *scroll = scroll.clamp(0.0, maxs);
    let mut y = r.y + pad - *scroll;
    for (x, t, size, f, c, h) in &lines {
        if *x < 0.0 {
            // marcador na mesma linha do próximo texto
            if y + 14.0 * s > r.y && y < r.y + r.h - 4.0 * s {
                ui.text(t, r.x + pad + 4.0 * s, y + 13.0 * s, *size, *f, *c);
            }
            continue;
        }
        if y + h > r.y + 2.0 * s && y + 13.0 * s < r.y + r.h - 2.0 * s && !t.is_empty() {
            ui.text(t, r.x + pad + x, y + 13.0 * s, *size, *f, *c);
        }
        y += h;
    }
    if maxs > 0.0 {
        let vis = r.h / (total + pad * 2.0);
        let bh = (r.h * vis).max(24.0 * s);
        let by = r.y + (r.h - bh) * (*scroll / maxs);
        draw_rectangle(r.x + r.w - 6.0 * s, by, 3.0 * s, bh, T.border);
    }
}

fn missao_blocks(sim: &Sim) -> Vec<B> {
    let m = tb::missao(sim);
    let mut v = vec![B::Dim(format!("{}  ·  horário do caso: {}", m.titulo, m.horario))];
    v.push(B::H("Objetivo".into()));
    v.push(B::P(m.objetivo));
    if !m.historico.is_empty() {
        v.push(B::H("Ocorrência".into()));
        v.push(B::P(m.historico));
    }
    if !m.quesitos.is_empty() {
        v.push(B::H("Quesitos".into()));
        for (i, q) in m.quesitos.iter().enumerate() {
            // os quesitos do caso já vêm numerados
            let numerado = q.trim_start().chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
            v.push(B::Li(if numerado { q.clone() } else { format!("{}. {}", i + 1, q) }));
        }
    }
    v.push(B::H("Autorizações".into()));
    for a in m.autorizacoes {
        v.push(if a.contains("NÃO") { B::Warn(a) } else { B::Li(a) });
    }
    if !m.briefing.is_empty() {
        v.push(B::H("Orientações recebidas".into()));
        for b in m.briefing {
            v.push(B::Li(b));
        }
    }
    v.push(B::H("Tarefas e progresso".into()));
    match m.contagem {
        Some((f, t)) => {
            v.push(B::P(format!("{} de {} etapas essenciais cumpridas.", f, t)));
            if m.tarefas.is_empty() {
                v.push(B::Dim("Neste nível as etapas não são listadas: consulte o POP em Apoio.".into()));
            }
            for (ok, l) in m.tarefas {
                v.push(B::Chk(ok, l));
            }
        }
        None => v.push(B::Dim("Nível sem orientação: as etapas e decisões são suas.".into())),
    }
    v
}

fn objeto_blocks(sim: &Sim, id: &str) -> Vec<B> {
    let Some(secs) = tb::objeto(sim, id) else { return vec![B::Dim("Objeto indisponível.".into())] };
    let mut v = vec![];
    for (k, sec) in secs.into_iter().enumerate() {
        if k > 0 {
            v.push(B::Gap);
        }
        v.push(B::H(sec.titulo.clone()));
        let warn = sec.titulo.starts_with("Cuidados");
        for l in sec.linhas {
            if warn || l.contains("DIVERG") || l.contains("rompido") || l.contains("LIGADO") {
                v.push(B::Warn(l));
            } else {
                v.push(B::Li(l));
            }
        }
    }
    v
}

const LOCAL: &str = "@LOCAL";

fn local_blocks(sim: &Sim) -> Vec<B> {
    use pericia_core::model::Setting;
    let local = sim.case.setting == Setting::Local;
    let mut v = vec![B::H("Ambiente".into())];
    v.push(B::P(if local {
        "Local de exame/apreensão. Documente o ambiente, reconheça e isole os equipamentos antes de intervir.".into()
    } else {
        "Laboratório. O material chegou lacrado; confira a documentação antes do deslacre.".into()
    }));
    let n = tb::itens_visiveis(sim).len();
    v.push(B::Li(format!("{} objeto(s) identificado(s) até agora", n)));
    let insp = sim.audit.entries.iter().any(|e| e.action == "inspecionar" && e.detail == "local");
    v.push(B::Chk(insp, if insp { "Busca minuciosa no local realizada".into() } else { "Busca minuciosa no local ainda não realizada".into() }));
    v
}

const CONTROLES: &str = "\
Deslocamento: W A S D (Shift corre) · olhar: botão direito ou Tab (captura o mouse)
Interagir com o objeto na mira: clique ou E — abre o painel de etapas do objeto
Maleta pericial: aproxime-se e use E — pegar ferramenta, calçar/trocar luvas, pulseira antiestática
Uma ferramenta por vez na mão; G devolve a ferramenta à maleta. Botões com ícone pedem a ferramenta certa
Tablet: T abre e fecha (Esc também fecha); nada muda no caso ao abrir ou fechar
Estação pericial: aproxime-se do computador da bancada e interaja
Terminal: botão Terminal na barra superior; 'ajuda' lista os comandos
Configurações (som, gráficos, controles, interface): botão ⚙ na barra superior ou Esc no ambiente";

fn apoio_blocks(sim: &Sim, sub: u8) -> Vec<B> {
    let mut v = vec![];
    let para = |v: &mut Vec<B>, t: &str| {
        for l in t.lines() {
            if l.trim().is_empty() {
                v.push(B::Gap);
            } else if l.chars().all(|c| !c.is_lowercase()) && l.chars().any(|c| c.is_alphabetic()) && l.len() < 90 {
                v.push(B::H(l.trim().to_string()));
            } else {
                v.push(B::P(l.to_string()));
            }
        }
    };
    match sub {
        0 => {
            v.push(B::Dim("Orientação do mentor — conforme o nível de dificuldade do caso.".into()));
            para(&mut v, &pericia_core::assess::hint(sim));
        }
        1 => para(&mut v, pericia_core::manual::pop_summary(sim.case.module)),
        2 => {
            v.push(B::Dim("Comandos do terminal (botão Terminal ou estação pericial). Use 'ajuda <tema>' para detalhes.".into()));
            para(&mut v, &pericia_core::tools::help(""));
        }
        _ => {
            for l in CONTROLES.lines() {
                v.push(B::Li(l.into()));
            }
        }
    }
    v
}

// ---------------------------------------------------------------- desenho

const TAB_NAMES: [&str; 4] = ["Missão", "Objeto", "Apoio", "Anotações"];

/// Entradas de teclado do tablet (chamada antes de desenhar a cena).
/// Retorna true se o tablet estiver aberto depois do quadro.
pub fn keys(app: &mut App, typing_elsewhere: bool) -> bool {
    let typing_note = app.ui.focus.as_deref() == Some("tnote");
    if app.phone.open.is_some() || app.sim.is_none() {
        return app.tablet.open;
    }
    if app.tablet.open {
        if app.ui.key_esc {
            if typing_note {
                app.ui.focus = None; // primeiro Esc só sai do campo de texto
            } else {
                close(app);
            }
        } else if !typing_note && is_key_pressed(KeyCode::T) {
            close(app);
        }
    } else if !typing_elsewhere && is_key_pressed(KeyCode::T) {
        open(app);
    }
    app.tablet.open
}

pub fn open(app: &mut App) {
    if app.tablet.item.is_none() {
        app.tablet.item = app.selected.clone().or_else(|| app.hover_pick.clone().filter(|p| p.starts_with("ITEM")));
    }
    if !app.tablet.open {
        app.audio.play("tablet_sacar", crate::audio::Cat::Interface, 1.0);
    }
    app.tablet.open = true;
    app.tablet.scroll = 0.0;
    app.player.set_captured(false);
}

pub fn close(app: &mut App) {
    if app.tablet.open {
        app.audio.play("tablet_guardar", crate::audio::Cat::Interface, 1.0);
    }
    app.tablet.open = false;
    if app.ui.focus.as_deref() == Some("tnote") {
        app.ui.focus = None;
    }
}

/// Dimensões do tablet em tela, com proporção do aparelho (≈ 26 × 18 cm).
fn body_rect(area: Rect, s: f32) -> Rect {
    let maxw = (area.w - 40.0 * s).min(1180.0 * s);
    let maxh = (area.h - 30.0 * s).min(820.0 * s);
    let (mut w, mut h) = (maxw, maxw / 1.42);
    if h > maxh {
        h = maxh;
        w = (h * 1.42).min(maxw);
    }
    rect(area.x + (area.w - w) / 2.0, area.y + (area.h - h) / 2.0, w, h)
}

pub fn draw(app: &mut App, area: Rect) {
    if !app.tablet.open || app.sim.is_none() {
        return;
    }
    let s = app.ui.scale;
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.0, 0.0, 0.0, 0.55));
    let body = body_rect(area, s);
    let u = body.h / 600.0;
    // corpo robusto: cantoneiras de borracha, moldura escura
    let orange = Color::new(0.86, 0.42, 0.10, 1.0);
    let corner = 70.0 * u;
    for (cx, cy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        rrect(rect(body.x - 6.0 * u + cx * (body.w - corner + 12.0 * u) , body.y - 6.0 * u + cy * (body.h - corner + 12.0 * u), corner, corner), 22.0 * u, orange);
    }
    rrect(body, 26.0 * u, Color::new(0.13, 0.14, 0.15, 1.0));
    rrect(rect(body.x + 3.0 * u, body.y + 3.0 * u, body.w - 6.0 * u, body.h - 6.0 * u), 24.0 * u, Color::new(0.07, 0.075, 0.08, 1.0));
    let scr = rect(body.x + 30.0 * u, body.y + 24.0 * u, body.w - 60.0 * u, body.h - 48.0 * u);
    draw_rectangle(scr.x, scr.y, scr.w, scr.h, Color::new(0.06, 0.075, 0.095, 1.0));
    draw_circle(body.x + 15.0 * u, body.y + body.h / 2.0, 3.0 * u, Color::new(0.02, 0.02, 0.03, 1.0)); // câmera
    // barra de status
    let (case_id, hora, nivel) = app
        .sim
        .as_ref()
        .map(|x| (x.case.id.clone(), pericia_core::util::fmt_dt(x.clock, pericia_core::util::BRT), x.case.difficulty))
        .unwrap();
    let sb = rect(scr.x, scr.y, scr.w, 26.0 * s);
    draw_rectangle(sb.x, sb.y, sb.w, sb.h, Color::new(0.03, 0.04, 0.05, 1.0));
    app.ui.text("TABLET PERICIAL", sb.x + 10.0 * s, sb.y + 18.0 * s, 11.0, F::Bold, orange);
    let info = format!("{} · nível {} · {}", case_id, nivel, hora);
    let iw = app.ui.measure(&info, 11.0, F::Reg);
    app.ui.text(&info, sb.x + (sb.w - iw) / 2.0, sb.y + 18.0 * s, 11.0, F::Reg, T.dim);
    let cr = rect(sb.x + sb.w - 120.0 * s, sb.y + 2.0 * s, 116.0 * s, sb.h - 4.0 * s);
    if app.ui.button_c(cr, "Guardar  (T)", Color::new(0.12, 0.14, 0.17, 1.0), true) {
        close(app);
        return;
    }
    // abas
    let ty = sb.y + sb.h + 6.0 * s;
    let th = 32.0 * s;
    let tw = ((scr.w - 20.0 * s) / 4.0).min(220.0 * s);
    for (k, name) in TAB_NAMES.iter().enumerate() {
        let label = if k == 3 && !app.tablet.notas.is_empty() { format!("{} ({})", name, app.tablet.notas.len()) } else { name.to_string() };
        if app.ui.tab(rect(scr.x + 10.0 * s + k as f32 * (tw + 4.0 * s), ty, tw, th), &label, app.tablet.tab == k as u8) && app.tablet.tab != k as u8 {
            app.tablet.tab = k as u8;
            app.tablet.scroll = 0.0;
        }
    }
    let content = rect(scr.x + 10.0 * s, ty + th + 8.0 * s, scr.w - 20.0 * s, scr.y + scr.h - (ty + th + 8.0 * s) - 10.0 * s);
    match app.tablet.tab {
        0 => {
            let b = missao_blocks(app.sim.as_ref().unwrap());
            app.ui.frame(content, T.term_bg, T.border);
            let mut sc = app.tablet.scroll;
            rich(&mut app.ui, content, &b, &mut sc);
            app.tablet.scroll = sc;
        }
        1 => draw_objeto(app, content),
        2 => draw_apoio(app, content),
        _ => draw_notas(app, content),
    }
    // mensagens efêmeras do tablet
    if let Some((m, until)) = app.tablet.msg.clone() {
        if get_time() < until {
            let w = app.ui.measure(&m, 12.0, F::Reg) + 24.0 * s;
            let r = rect(scr.x + scr.w - w - 130.0 * s, scr.y + 1.0 * s, w, 24.0 * s);
            app.ui.frame(r, Color::new(0.05, 0.08, 0.12, 0.96), orange);
            app.ui.text(&m, r.x + 12.0 * s, r.y + 17.0 * s, 12.0, F::Reg, T.text);
        } else {
            app.tablet.msg = None;
        }
    }
    // o tablet consome cliques e rolagem: nada passa para a cena
    app.ui.click_consumed = true;
    app.ui.wheel = 0.0;
}

fn draw_objeto(app: &mut App, r: Rect) {
    let s = app.ui.scale;
    let sim = app.sim.as_ref().unwrap();
    let mut itens = vec![(LOCAL.to_string(), "Local de exame".to_string())];
    itens.extend(tb::itens_visiveis(sim));
    if app.tablet.item.as_ref().map(|i| !itens.iter().any(|(x, _)| x == i)).unwrap_or(true) {
        app.tablet.item = itens.first().map(|x| x.0.clone());
    }
    let lw = (r.w * 0.28).clamp(170.0 * s, 280.0 * s);
    let mut y = r.y;
    app.ui.text("Objetos disponíveis", r.x, y + 12.0 * s, 11.0, F::Bold, T.dim);
    y += 20.0 * s;
    for (id, label) in &itens {
        let full = if id == LOCAL { format!("◇ {}", label) } else { format!("{} — {}", id, label) };
        let txt = clip_text(&app.ui, &full, 12.0, F::Bold, lw - 16.0 * s);
        if app.ui.tab(rect(r.x, y, lw, 30.0 * s), &txt, app.tablet.item.as_deref() == Some(id.as_str())) {
            app.tablet.item = Some(id.clone());
            app.tablet.scroll = 0.0;
        }
        y += 34.0 * s;
        if y > r.y + r.h - 80.0 * s {
            break;
        }
    }
    let dr = rect(r.x + lw + 10.0 * s, r.y, r.w - lw - 10.0 * s, r.h);
    let Some(id) = app.tablet.item.clone() else {
        app.ui.frame(dr, T.term_bg, T.border);
        app.ui.text("Nenhum objeto disponível.", dr.x + 12.0 * s, dr.y + 24.0 * s, 12.0, F::Reg, T.dim);
        return;
    };
    let b = if id == LOCAL { local_blocks(app.sim.as_ref().unwrap()) } else { objeto_blocks(app.sim.as_ref().unwrap(), &id) };
    let bar = 40.0 * s;
    let tr = rect(dr.x, dr.y, dr.w, dr.h - bar);
    app.ui.frame(tr, T.term_bg, T.border);
    let mut sc = app.tablet.scroll;
    rich(&mut app.ui, tr, &b, &mut sc);
    app.tablet.scroll = sc;
    let br = rect(dr.x, dr.y + dr.h - bar + 6.0 * s, 260.0 * s, bar - 6.0 * s);
    if app.ui.button_c(br, "Selecionar e abrir ações  ▸", T.accent2, true) {
        if id == LOCAL {
            app.selected = None;
            app.show_help_overlay = true;
        } else {
            app.selected = Some(id);
        }
        app.panel_scroll = 0.0;
        close(app);
    }
    app.ui.text("A ficha é só leitura: as ações ficam no painel do objeto.", br.x + br.w + 12.0 * s, br.y + 21.0 * s, 10.5, F::Reg, T.dim);
}

fn draw_apoio(app: &mut App, r: Rect) {
    let s = app.ui.scale;
    let subs = ["Mentor", "Resumo do POP", "Comandos", "Controles"];
    let mut x = r.x;
    for (k, n) in subs.iter().enumerate() {
        let w = app.ui.measure(n, 13.0, F::Bold) + 26.0 * s;
        if app.ui.tab(rect(x, r.y, w, 26.0 * s), n, app.tablet.sub == k as u8) && app.tablet.sub != k as u8 {
            app.tablet.sub = k as u8;
            app.tablet.scroll = 0.0;
        }
        x += w + 4.0 * s;
    }
    let cr = rect(r.x, r.y + 32.0 * s, r.w, r.h - 32.0 * s);
    let b = apoio_blocks(app.sim.as_ref().unwrap(), app.tablet.sub);
    app.ui.frame(cr, T.term_bg, T.border);
    let mut sc = app.tablet.scroll;
    rich(&mut app.ui, cr, &b, &mut sc);
    app.tablet.scroll = sc;
}

fn tag_color(tag: &str) -> Color {
    match tag {
        "Hipótese" => Color::new(0.62, 0.45, 0.95, 1.0),
        "Horário" => Color::new(0.30, 0.70, 0.95, 1.0),
        "Identificador" => Color::new(0.35, 0.80, 0.55, 1.0),
        "Pendência" => Color::new(0.98, 0.70, 0.25, 1.0),
        _ => Color::new(0.75, 0.78, 0.82, 1.0),
    }
}

fn draw_notas(app: &mut App, r: Rect) {
    let s = app.ui.scale;
    let clock = app.sim.as_ref().map(|x| x.clock).unwrap_or(0);
    let lw = (r.w * 0.36).clamp(220.0 * s, 380.0 * s);
    // lista
    let lr = rect(r.x, r.y + 34.0 * s, lw, r.h - 34.0 * s);
    if app.ui.button_c(rect(r.x, r.y, lw, 28.0 * s), "+  Nova anotação", T.accent2, true) {
        app.tablet.new_draft();
        app.ui.focus = Some("tnote".into());
    }
    app.ui.frame(lr, T.term_bg, T.border);
    if app.tablet.notas.is_empty() {
        for (k, l) in app.ui.wrap("Nenhuma anotação. As anotações são suas: ficam guardadas com o caso, separadas das evidências, da auditoria e da avaliação.", 11.5, F::Reg, lw - 20.0 * s).iter().enumerate() {
            app.ui.text(l, lr.x + 10.0 * s, lr.y + 20.0 * s + k as f32 * 16.0 * s, 11.5, F::Reg, T.dim);
        }
    }
    let ch = 62.0 * s;
    let total = app.tablet.notas.len() as f32 * (ch + 4.0 * s);
    let maxs = (total - lr.h + 8.0 * s).max(0.0);
    if app.ui.hover(lr) && app.ui.wheel != 0.0 {
        app.tablet.list_scroll -= app.ui.wheel.signum() * 40.0 * s;
    }
    app.tablet.list_scroll = app.tablet.list_scroll.clamp(0.0, maxs);
    let mut y = lr.y + 4.0 * s - app.tablet.list_scroll;
    let mut pick = None;
    for n in app.tablet.notas.iter().rev() {
        let cr = rect(lr.x + 4.0 * s, y, lr.w - 8.0 * s, ch);
        y += ch + 4.0 * s;
        if cr.y < lr.y || cr.y + cr.h > lr.y + lr.h {
            continue;
        }
        let sel = app.tablet.edit == Some(n.id);
        app.ui.frame(cr, if sel { T.panel2 } else { T.panel }, if sel { T.accent } else { T.border });
        draw_rectangle(cr.x, cr.y, 4.0 * s, cr.h, tag_color(&n.tag));
        let hdr = format!("{} · {}{}", n.tag, pericia_core::util::fmt_dt(n.criada, pericia_core::util::BRT), if n.editada.is_some() { " · editada" } else { "" });
        app.ui.text(&hdr, cr.x + 10.0 * s, cr.y + 15.0 * s, 10.5, F::Bold, tag_color(&n.tag));
        for (k, l) in app.ui.wrap(&n.texto.replace('\n', " "), 11.5, F::Reg, cr.w - 20.0 * s).iter().take(2).enumerate() {
            app.ui.text(l, cr.x + 10.0 * s, cr.y + 33.0 * s + k as f32 * 15.0 * s, 11.5, F::Reg, T.text);
        }
        if app.ui.click_in(cr) {
            pick = Some(n.id);
        }
    }
    if let Some(id) = pick {
        app.tablet.load_note(id);
    }
    // editor
    let er = rect(r.x + lw + 10.0 * s, r.y, r.w - lw - 10.0 * s, r.h);
    let titulo = match app.tablet.edit {
        Some(id) => format!("Editando a anotação nº {}", id),
        None => "Nova anotação".to_string(),
    };
    app.ui.text(&titulo, er.x, er.y + 14.0 * s, 13.0, F::Bold, T.text);
    let mut x = er.x;
    let ty = er.y + 24.0 * s;
    for (k, t) in TAGS.iter().enumerate() {
        let w = app.ui.measure(t, 13.0, F::Reg) + 20.0 * s;
        let on = app.tablet.draft_tag == k;
        if app.ui.button_c(rect(x, ty, w, 26.0 * s), t, if on { Color::new(tag_color(t).r * 0.45, tag_color(t).g * 0.45, tag_color(t).b * 0.45, 1.0) } else { T.panel }, true) {
            app.tablet.draft_tag = k;
        }
        if on {
            draw_rectangle(x, ty + 26.0 * s - 3.0 * s, w, 3.0 * s, tag_color(t));
        }
        x += w + 4.0 * s;
    }
    let bar = 38.0 * s;
    let ar = rect(er.x, ty + 34.0 * s, er.w, er.h - (ty + 34.0 * s - er.y) - bar);
    let mut buf = std::mem::take(&mut app.tablet.draft);
    let mut sc = app.tablet.draft_scroll;
    app.ui.text_area("tnote", ar, &mut buf, &mut sc);
    app.tablet.draft = buf;
    app.tablet.draft_scroll = sc;
    if app.tablet.draft.is_empty() && app.ui.focus.as_deref() != Some("tnote") {
        app.ui.text("Clique aqui e escreva. Ex.: “14h32 — LED de rede piscando no gabinete”.", ar.x + 8.0 * s, ar.y + 20.0 * s, 12.0, F::Reg, T.dim);
    }
    let by = er.y + er.h - bar + 6.0 * s;
    let bh = bar - 6.0 * s;
    if app.ui.button_c(rect(er.x, by, 130.0 * s, bh), "Gravar", T.accent2, !app.tablet.draft.trim().is_empty()) {
        app.tablet.save_draft(clock);
    }
    if let Some(id) = app.tablet.edit {
        let now = get_time();
        let armed = app.tablet.confirm_del.map(|(i, t)| i == id && now < t).unwrap_or(false);
        let label = if armed { "Confirmar exclusão" } else { "Excluir" };
        if app.ui.button_c(rect(er.x + 136.0 * s, by, 160.0 * s, bh), label, if armed { Color::new(0.55, 0.16, 0.15, 1.0) } else { T.panel2 }, true) {
            if armed {
                app.tablet.confirm_del = None;
                app.tablet.delete(id);
            } else {
                app.tablet.confirm_del = Some((id, now + 4.0));
            }
        }
    }
    app.ui.text("Não altera evidências nem a avaliação.", er.x + 304.0 * s, by + 20.0 * s, 10.5, F::Reg, T.dim);
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn json_ida_e_volta() {
        let n = vec![
            Nota { id: 1, tag: "Hipótese".into(), texto: "linha 1\nlinha \"2\"".into(), criada: 100, editada: None },
            Nota { id: 3, tag: "Horário".into(), texto: "x".into(), criada: 200, editada: Some(260) },
        ];
        let j = notes_to_json("5.01-D2-0042", &n);
        assert_eq!(notes_from_json("5.01-D2-0042", &j), n);
        // anotações de outro caso não vazam para o caso atual
        assert!(notes_from_json("5.02-D1-0001", &j).is_empty());
        assert!(notes_from_json("x", "lixo").is_empty());
    }
}
