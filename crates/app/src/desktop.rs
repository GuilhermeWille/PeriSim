//! "PeritOS": a área de trabalho da Estação Pericial. Cada aplicativo apenas monta
//! comandos do terminal do simulador — assim tudo passa pelo motor e pela auditoria,
//! e o usuário aprende a sintaxe vendo os comandos aparecerem no terminal.

use crate::ui::*;
use crate::App;
use macroquad::prelude::*;
use pericia_core::model::*;

#[derive(Clone, Copy, PartialEq)]
pub enum DApp {
    Caso,
    Terminal,
    Evidencias,
    Navegador,
    Custodia,
    Linha,
    Laudo,
    Manual,
    Mentor,
    Avaliacao,
}

impl DApp {
    pub const ALL: [DApp; 10] = [DApp::Caso, DApp::Terminal, DApp::Evidencias, DApp::Navegador, DApp::Custodia, DApp::Linha, DApp::Laudo, DApp::Manual, DApp::Mentor, DApp::Avaliacao];
    pub fn name(self) -> &'static str {
        match self {
            DApp::Caso => "Caso",
            DApp::Terminal => "Terminal",
            DApp::Evidencias => "Evidências",
            DApp::Navegador => "Navegador",
            DApp::Custodia => "Custódia",
            DApp::Linha => "Linha do tempo",
            DApp::Laudo => "Documento",
            DApp::Manual => "Manual POP",
            DApp::Mentor => "Mentor",
            DApp::Avaliacao => "Avaliação",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            DApp::Caso => "§",
            DApp::Terminal => ">_",
            DApp::Evidencias => "▤",
            DApp::Navegador => "◎",
            DApp::Custodia => "⛓",
            DApp::Linha => "≡",
            DApp::Laudo => "✎",
            DApp::Manual => "?",
            DApp::Mentor => "☼",
            DApp::Avaliacao => "✓",
        }
    }
}

pub struct DesktopState {
    pub app: DApp,
    pub scroll: f32,
    pub ev_sel: Option<(String, String)>,
    pub ev_out: String,
    pub ev_out_scroll: f32,
    pub ev_list_scroll: f32,
    pub ev_pass: String,
    pub ev_search: String,
    pub nav_url: String,
    pub nav_view: String,
    pub nav_links: Vec<String>,
    pub nav_scroll: f32,
    pub nav_tool: String,
    pub laudo: [String; 6],
    pub laudo_scroll: [f32; 6],
    pub laudo_loaded: bool,
    pub laudo_msg: String,
    pub manual_tab: ModuleId,
    pub eval_text: String,
}

pub const LAUDO_FIELDS: [(&str, &str); 6] = [
    ("material", "Material"),
    ("exame", "Exame"),
    ("origem", "Origem da expressão de interesse"),
    ("consideracoes", "Considerações técnicas e limitações"),
    ("conclusao", "Conclusão"),
    ("hipoteses", "Hipóteses não confirmadas (opcional)"),
];

impl DesktopState {
    pub fn new() -> Self {
        DesktopState {
            app: DApp::Caso,
            scroll: 0.0,
            ev_sel: None,
            ev_out: String::new(),
            ev_out_scroll: 0.0,
            ev_list_scroll: 0.0,
            ev_pass: String::new(),
            ev_search: String::new(),
            nav_url: String::new(),
            nav_view: "Digite um endereço da rede emulada e pressione Enter.\nNenhuma requisição sai deste computador: a internet do caso é simulada.".into(),
            nav_links: vec![],
            nav_scroll: 0.0,
            nav_tool: String::new(),
            laudo: Default::default(),
            laudo_scroll: [0.0; 6],
            laudo_loaded: false,
            laudo_msg: String::new(),
            manual_tab: ModuleId::M501,
            eval_text: String::new(),
        }
    }
}

/// Entrada visível no navegador de evidências.
struct EvEntry {
    media: String,
    path: String,
    label: String,
    deleted: bool,
    kind: &'static str,
}

fn evidence_entries(app: &App) -> Vec<EvEntry> {
    let sim = app.sim.as_ref().unwrap();
    let open = sim.open_set();
    let mut out = vec![];
    fn walk(files: &[VFile], open: &std::collections::BTreeSet<String>, mid: &str, depth: usize, out: &mut Vec<EvEntry>) {
        for f in files {
            let kind = if f.container.is_some() {
                "cifrado"
            } else if !f.children.is_empty() {
                "composto"
            } else {
                f.ftype.label()
            };
            out.push(EvEntry { media: mid.into(), path: f.path.clone(), label: format!("{}{}", "  ".repeat(depth), f.path), deleted: f.deleted, kind });
            if !f.children.is_empty() && open.contains(&f.id) {
                walk(&f.children, open, mid, depth + 1, out);
            }
            if let Some(c) = &f.container {
                if open.contains(&c.id) {
                    walk(&c.files, open, mid, depth + 1, out);
                }
            }
        }
    }
    for m in sim.case.media.iter().filter(|m| sim.imaged.contains(&m.id)) {
        for p in &m.partitions {
            if p.encrypted.is_some() && !sim.decrypted.contains(&p.id) {
                out.push(EvEntry { media: m.id.clone(), path: p.id.clone(), label: format!("[{}] partição {} — CIFRADA", m.id, p.label), deleted: false, kind: "volume" });
                continue;
            }
            out.push(EvEntry { media: m.id.clone(), path: String::new(), label: format!("[{}] {} ({}) — +{} arquivos de sistema", m.id, p.label, p.fs, p.noise), deleted: false, kind: "cabecalho" });
            walk(&p.files, &open, &m.id, 1, &mut out);
        }
    }
    out
}

#[allow(dead_code)]
pub fn draw(app: &mut App, area: Rect) {
    let ui_s = app.ui.scale;
    let side_w = 150.0 * ui_s;
    // fundo "papel de parede"
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.09, 0.12, 0.17, 1.0));
    // lançador
    let side = rect(area.x, area.y, side_w, area.h);
    app.ui.panel(side, T.panel);
    app.ui.text("PeritOS", side.x + app.ui.s(12.0), side.y + app.ui.s(24.0), 15.0, F::Bold, T.accent);
    app.ui.text("rede isolada", side.x + app.ui.s(12.0), side.y + app.ui.s(40.0), 9.5, F::Reg, T.dim);
    let mut y = side.y + app.ui.s(52.0);
    let bh = app.ui.s(30.0);
    for a in DApp::ALL {
        let r = rect(side.x + app.ui.s(6.0), y, side_w - app.ui.s(12.0), bh);
        let active = app.desk.app == a;
        let fill = if active { T.accent2 } else if app.ui.hover(r) { T.panel2 } else { T.panel };
        app.ui.frame(r, fill, if active { T.accent } else { fill });
        app.ui.text(a.icon(), r.x + app.ui.s(8.0), r.y + bh * 0.66, 13.0, F::Mono, T.accent);
        app.ui.text(a.name(), r.x + app.ui.s(34.0), r.y + bh * 0.66, 12.5, F::Reg, T.text);
        if app.ui.click_in(r) {
            switch(app, a);
        }
        y += bh + app.ui.s(3.0);
    }
    let main = rect(area.x + side_w + app.ui.s(8.0), area.y + app.ui.s(8.0), area.w - side_w - app.ui.s(16.0), area.h - app.ui.s(16.0));
    app.ui.frame(main, T.panel, T.border);
    let title = rect(main.x, main.y, main.w, app.ui.s(28.0));
    app.ui.panel(title, T.panel2);
    app.ui.text(app.desk.app.name(), title.x + app.ui.s(10.0), title.y + app.ui.s(19.0), 13.0, F::Bold, T.text);
    let body = rect(main.x + app.ui.s(8.0), main.y + app.ui.s(36.0), main.w - app.ui.s(16.0), main.h - app.ui.s(44.0));
    match app.desk.app {
        DApp::Caso => {
            let t = app.sim.as_ref().map(|s| pericia_core::tools::describe_case(s)).unwrap_or_default();
            let mut sc = app.desk.scroll;
            app.ui.scroll_text(body, &t, &mut sc, 12.5, F::Reg, T.text, false);
            app.desk.scroll = sc;
        }
        DApp::Terminal => crate::draw_terminal(app, body, true),
        DApp::Evidencias => evidencias(app, body),
        DApp::Navegador => navegador(app, body),
        DApp::Custodia => {
            let t = app.sim.as_ref().map(|s| format!("{}\n\nAUDITORIA ENCADEADA (SHA-256)\n{}", pericia_core::tools::custody(s), s.audit.render(pericia_core::util::BRT))).unwrap_or_default();
            let mut sc = app.desk.scroll;
            app.ui.scroll_text(body, &t, &mut sc, 11.5, F::Mono, T.term_fg, false);
            app.desk.scroll = sc;
        }
        DApp::Linha => {
            let t = app.sim.as_ref().map(pericia_core::tools::timeline_text).unwrap_or_default();
            let mut sc = app.desk.scroll;
            app.ui.scroll_text(body, &format!("{}\n\nDica: a linha do tempo inclui apenas o que você já examinou (imagens montadas, WHOIS, registros de conexão, cabeçalhos).", t), &mut sc, 11.5, F::Mono, T.term_fg, false);
            app.desk.scroll = sc;
        }
        DApp::Laudo => laudo(app, body),
        DApp::Manual => manual(app, body),
        DApp::Mentor => {
            let t = app.sim.as_ref().map(pericia_core::assess::hint).unwrap_or_default();
            let mut sc = app.desk.scroll;
            app.ui.scroll_text(body, &t, &mut sc, 13.0, F::Reg, T.text, false);
            app.desk.scroll = sc;
        }
        DApp::Avaliacao => {
            let t = if app.desk.eval_text.is_empty() {
                let prog = app.sim.as_ref().map(pericia_core::assess::progress).unwrap_or_default();
                format!("O caso ainda não foi finalizado.\nA avaliação detalhada aparece após 'Emitir e finalizar' no aplicativo Documento.\n\n{}", prog)
            } else {
                app.desk.eval_text.clone()
            };
            let mut sc = app.desk.scroll;
            app.ui.scroll_text(body, &t, &mut sc, 12.5, F::Reg, T.text, false);
            app.desk.scroll = sc;
        }
    }
}

pub fn switch(app: &mut App, a: DApp) {
    if app.desk.app == DApp::Laudo && a != DApp::Laudo {
        sync_laudo(app);
    }
    app.desk.app = a;
    app.desk.scroll = 0.0;
    app.ui.focus = if a == DApp::Terminal { Some("term".into()) } else { None };
    if a == DApp::Laudo {
        load_laudo(app);
    }
}

pub fn evidencias(app: &mut App, body: Rect) {
    let entries = evidence_entries(app);
    let s = app.ui.scale;
    let list = rect(body.x, body.y, body.w * 0.46, body.h);
    let right = rect(list.x + list.w + 8.0 * s, body.y, body.w - list.w - 8.0 * s, body.h);
    app.ui.frame(list, T.term_bg, T.border);
    if entries.is_empty() {
        app.ui.text("Nenhuma imagem adquirida ainda.", list.x + 10.0 * s, list.y + 24.0 * s, 12.5, F::Reg, T.dim);
        app.ui.text("Adquira na Área de Hardware ('Adquirir imagem' / 'Extrair').", list.x + 10.0 * s, list.y + 44.0 * s, 11.5, F::Reg, T.dim);
        app.ui.text("O exame é sempre feito sobre a cópia (POP 5.01).", list.x + 10.0 * s, list.y + 62.0 * s, 11.5, F::Reg, T.dim);
    }
    let lh = 20.0 * s;
    let visible = ((list.h - 8.0 * s) / lh) as usize;
    let maxs = entries.len().saturating_sub(visible) as f32;
    if app.ui.hover(list) && app.ui.wheel != 0.0 {
        app.desk.ev_list_scroll = (app.desk.ev_list_scroll - app.ui.wheel.signum() * 3.0).clamp(0.0, maxs);
    }
    let start = app.desk.ev_list_scroll.min(maxs) as usize;
    for (i, e) in entries.iter().skip(start).take(visible).enumerate() {
        let r = rect(list.x + 2.0, list.y + 4.0 * s + i as f32 * lh, list.w - 4.0, lh);
        let sel = app.desk.ev_sel.as_ref().map(|(m, p)| m == &e.media && p == &e.path).unwrap_or(false);
        if sel {
            app.ui.panel(r, T.accent2);
        } else if app.ui.hover(r) && e.kind != "cabecalho" {
            app.ui.panel(r, T.panel2);
        }
        let col = if e.kind == "cabecalho" { T.accent } else if e.deleted { T.warn } else if e.kind == "cifrado" || e.kind == "volume" { T.bad } else { T.text };
        let mut label = format!("{}{}", if e.deleted { "✗ " } else { "" }, e.label);
        if e.kind != "cabecalho" {
            label = format!("{}  [{}]", label, e.kind);
        }
        let maxc = ((list.w - 16.0 * s) / app.ui.mono_w * 1.18) as usize;
        if label.chars().count() > maxc {
            label = label.chars().take(maxc.saturating_sub(1)).collect::<String>() + "…";
        }
        app.ui.text(&label, r.x + 6.0 * s, r.y + lh * 0.72, 11.5, F::Mono, col);
        if e.kind != "cabecalho" && app.ui.click_in(r) {
            app.desk.ev_sel = Some((e.media.clone(), e.path.clone()));
        }
    }
    // painel de ações
    let mut y = right.y;
    let bh = 28.0 * s;
    let search_r = rect(right.x, y, right.w - 96.0 * s, bh);
    let go = app.ui.input("ev_search", search_r, &mut app.desk.ev_search, "buscar termo nas imagens (fs buscar)");
    let b = rect(right.x + right.w - 90.0 * s, y, 90.0 * s, bh);
    if (app.ui.button(b, "Buscar") || go) && !app.desk.ev_search.trim().is_empty() {
        let c = format!("fs buscar {}", app.desk.ev_search.trim());
        app.desk.ev_out = app.run(&c);
        app.desk.ev_out_scroll = 0.0;
    }
    y += bh + 8.0 * s;
    if let Some((mid, path)) = app.desk.ev_sel.clone() {
        let is_vol = !path.starts_with('/') && path.contains(':') && !path.contains('.');
        app.ui.text(&format!("Selecionado: {} {}", mid, path), right.x, y + 14.0 * s, 11.5, F::Mono, T.dim);
        y += 22.0 * s;
        let acts: Vec<(&str, String)> = if is_vol {
            vec![]
        } else {
            vec![
                ("Ler", format!("fs cat {}", path)),
                ("Metadados", format!("fs meta {}", path)),
                ("Recuperar", format!("recuperar {} {}", mid, path)),
                ("Expandir", format!("expandir {} {}", mid, path)),
                ("OCR", format!("ocr {} {}", mid, path)),
                ("Hash", format!("hash {}", path)),
            ]
        };
        let bw = (right.w - 10.0 * s) / 3.0;
        for (i, (l, c)) in acts.iter().enumerate() {
            let r = rect(right.x + (i % 3) as f32 * (bw + 5.0 * s), y + (i / 3) as f32 * (bh + 5.0 * s), bw, bh);
            if app.ui.button(r, l) {
                app.desk.ev_out = app.run(c);
                app.desk.ev_out_scroll = 0.0;
            }
        }
        if !acts.is_empty() {
            y += 2.0 * (bh + 5.0 * s);
        }
        let pr = rect(right.x, y, right.w - 186.0 * s, bh);
        let enter = app.ui.input("ev_pass", pr, &mut app.desk.ev_pass, "senha para decifrar");
        if (app.ui.button(rect(right.x + right.w - 180.0 * s, y, 86.0 * s, bh), "Decifrar") || enter) && !app.desk.ev_pass.is_empty() {
            let c = format!("decifrar {} {}", path, app.desk.ev_pass);
            app.desk.ev_out = app.run(&c);
        }
        if app.ui.button(rect(right.x + right.w - 88.0 * s, y, 88.0 * s, bh), "Dicionário") {
            app.desk.ev_out = app.run("dicionario item-01");
        }
        y += bh + 6.0 * s;
    }
    let row = rect(right.x, y, right.w, bh);
    let bw = (row.w - 10.0 * s) / 3.0;
    if app.ui.button(rect(row.x, row.y, bw, bh), "Senhas no termo") {
        app.desk.ev_out = app.run("senha");
    }
    if app.ui.button(rect(row.x + bw + 5.0 * s, row.y, bw, bh), "Linha do tempo") {
        app.desk.ev_out = app.run("linha-do-tempo");
    }
    if app.ui.button(rect(row.x + 2.0 * (bw + 5.0 * s), row.y, bw, bh), "Anotar seleção") {
        if let Some((m, p)) = app.desk.ev_sel.clone() {
            let c = format!("anotar Examinado {} na mídia {}", p, m);
            app.desk.ev_out = app.run(&c);
        }
    }
    y += bh + 8.0 * s;
    let out_r = rect(right.x, y, right.w, right.y + right.h - y);
    let t = app.desk.ev_out.clone();
    let mut sc = app.desk.ev_out_scroll;
    app.ui.scroll_text(out_r, &t, &mut sc, 11.5, F::Mono, T.term_fg, false);
    app.desk.ev_out_scroll = sc;
}

pub fn navegador(app: &mut App, body: Rect) {
    let s = app.ui.scale;
    let bh = 28.0 * s;
    let has_net = app.sim.as_ref().map(|x| x.case.net.is_some() || !x.case.provided.is_empty()).unwrap_or(false);
    // conexão
    let conn = app.sim.as_ref().map(|x| x.net_conn.clone()).unwrap_or_default();
    app.ui.text("Conexão:", body.x, body.y + 18.0 * s, 12.0, F::Reg, T.dim);
    let mut x = body.x + 64.0 * s;
    for (k, l) in [("institucional", "Institucional"), ("dedicada", "Dedicada"), ("anonima", "Anônima")] {
        let r = rect(x, body.y, 108.0 * s, bh);
        if app.ui.tab(r, l, conn == k) {
            let c = format!("conexao {}", k);
            app.nav_message(&c);
        }
        x += 112.0 * s;
    }
    let ip = app.sim.as_ref().and_then(|x| x.case.net.as_ref().and_then(|n| n.expert_ips.get(&x.net_conn).cloned())).unwrap_or_default();
    app.ui.text(&format!("IP público usado: {}", ip), x + 8.0 * s, body.y + 18.0 * s, 11.5, F::Mono, T.dim);
    // barra de endereço
    let y = body.y + bh + 6.0 * s;
    let url_r = rect(body.x, y, body.w - 300.0 * s, bh);
    let enter = app.ui.input("nav_url", url_r, &mut app.desk.nav_url, "https://exemplo.sim/");
    let mut bx = url_r.x + url_r.w + 4.0 * s;
    if app.ui.button(rect(bx, y, 70.0 * s, bh), "Ir") || enter {
        let u = app.desk.nav_url.trim().to_string();
        if !u.is_empty() {
            let c = format!("navegar {}", u);
            let out = app.run(&c);
            set_page(app, out);
        }
    }
    bx += 74.0 * s;
    if app.ui.button(rect(bx, y, 108.0 * s, bh), "Preservar") {
        let c = format!("preservar {}", app.desk.nav_url.trim());
        let out = app.run(&c);
        app.desk.nav_view = format!("{}\n\n{}", out, app.desk.nav_view);
    }
    bx += 112.0 * s;
    if app.ui.button(rect(bx, y, 108.0 * s, bh), "Histórico") {
        let c = format!("arquivo-web {}", app.desk.nav_url.trim());
        let out = app.run(&c);
        set_page(app, out);
    }
    // ferramentas
    let y2 = y + bh + 6.0 * s;
    let tr = rect(body.x, y2, body.w * 0.40, bh);
    app.ui.input("nav_tool", tr, &mut app.desk.nav_tool, "domínio / IP / arquivo .eml");
    let mut bx = tr.x + tr.w + 4.0 * s;
    let arg = app.desk.nav_tool.trim().to_string();
    for (l, c) in [("DNS", "dns"), ("WHOIS", "whois"), ("Traceroute", "traceroute"), ("Logs ISP", "rede logs"), ("Cabeçalho", "email cabecalho")] {
        let w = (body.w * 0.6 - 24.0 * s) / 5.0;
        if app.ui.button(rect(bx, y2, w, bh), l) && !arg.is_empty() {
            let cmd = format!("{} {}", c, arg);
            let out = app.run(&cmd);
            set_page(app, out);
        }
        bx += w + 4.0 * s;
    }
    let mut y3 = y2 + bh + 6.0 * s;
    // atalhos para material encaminhado
    let provided: Vec<String> = app.sim.as_ref().map(|x| x.case.provided.iter().map(|f| f.name().to_string()).collect()).unwrap_or_default();
    if !provided.is_empty() {
        app.ui.text("Material encaminhado:", body.x, y3 + 15.0 * s, 11.5, F::Reg, T.dim);
        let mut bx = body.x + 150.0 * s;
        for p in provided {
            let w = app.ui.measure(&p, 13.0, F::Reg) + 20.0 * s;
            if app.ui.button(rect(bx, y3, w, bh * 0.85), &p) {
                app.desk.nav_tool = p.clone();
                let out = app.run(&format!("email cabecalho {}", p));
                set_page(app, out);
            }
            bx += w + 4.0 * s;
        }
        y3 += bh;
    }
    // links da página
    if !app.desk.nav_links.is_empty() {
        app.ui.text("Links:", body.x, y3 + 15.0 * s, 11.5, F::Reg, T.dim);
        let mut bx = body.x + 50.0 * s;
        for l in app.desk.nav_links.clone() {
            let w = app.ui.measure(&l, 13.0, F::Reg) + 16.0 * s;
            if bx + w > body.x + body.w {
                break;
            }
            if app.ui.button_c(rect(bx, y3, w, bh * 0.85), &l, T.accent2, true) {
                app.desk.nav_url = l.clone();
                let out = app.run(&format!("navegar {}", l));
                set_page(app, out);
            }
            bx += w + 4.0 * s;
        }
        y3 += bh;
    }
    let view = rect(body.x, y3 + 2.0 * s, body.w, body.y + body.h - y3 - 2.0 * s);
    if !has_net {
        app.desk.nav_view = "Este caso não possui componente de internet. O navegador opera somente sobre a rede emulada do caso.".into();
    }
    let t = app.desk.nav_view.clone();
    let mut sc = app.desk.nav_scroll;
    app.ui.scroll_text(view, &t, &mut sc, 12.5, F::Reg, T.text, false);
    app.desk.nav_scroll = sc;
}

fn set_page(app: &mut App, out: String) {
    app.desk.nav_links = out
        .lines()
        .find(|l| l.starts_with("Links:"))
        .map(|l| l.trim_start_matches("Links:").split_whitespace().map(|x| x.to_string()).collect())
        .unwrap_or_default();
    app.desk.nav_view = out;
    app.desk.nav_scroll = 0.0;
}

pub fn load_laudo(app: &mut App) {
    if let Some(s) = &app.sim {
        let r = &s.report;
        app.desk.laudo = [r.material.clone(), r.exame.clone(), r.keyword_source.clone(), r.consideracoes.clone(), r.conclusao.clone(), r.hipoteses.clone()];
        app.desk.laudo_loaded = true;
    }
}

/// Envia ao motor os campos alterados no editor (como comandos, para manter o replay).
pub fn sync_laudo(app: &mut App) {
    let Some(s) = &app.sim else { return };
    let r = &s.report;
    let cur = [r.material.clone(), r.exame.clone(), r.keyword_source.clone(), r.consideracoes.clone(), r.conclusao.clone(), r.hipoteses.clone()];
    let fields = app.desk.laudo.clone();
    for (i, (k, _)) in LAUDO_FIELDS.iter().enumerate() {
        if fields[i] != cur[i] {
            let v = fields[i].replace('\n', " ¶ ");
            app.run_quiet(&format!("laudo {} {}", k, v));
        }
    }
}

pub fn laudo(app: &mut App, body: Rect) {
    let s = app.ui.scale;
    let Some(sim) = &app.sim else { return };
    let doc = sim.case.module.document();
    let kw_needed = !sim.case.keyword.is_empty();
    let claim = sim.report.keyword_claim.clone();
    let submitted = sim.report.submitted;
    let bh = 28.0 * s;
    // barra de ações
    let acts = ["Pré-preencher", "Verificar coerência", "Pré-visualizar", "Exportar PDF + MD", "Emitir e finalizar"];
    let bw = (body.w - 4.0 * 4.0 * s) / 5.0;
    for (i, l) in acts.iter().enumerate() {
        let r = rect(body.x + i as f32 * (bw + 4.0 * s), body.y, bw, bh);
        let base = if i == 4 { Color::new(0.16, 0.45, 0.28, 1.0) } else { T.panel2 };
        if app.ui.button_c(r, l, base, !(i == 4 && submitted)) {
            sync_laudo(app);
            match i {
                0 => {
                    app.run("laudo auto");
                    load_laudo(app);
                    app.desk.laudo_msg = "Campos vazios pré-preenchidos a partir da auditoria e das anotações. REVISE antes de emitir.".into();
                }
                1 => {
                    let c = pericia_core::report::check_coherence(app.sim.as_ref().unwrap());
                    app.desk.laudo_msg = if c.problems.is_empty() { "Nenhuma incoerência detectada entre o texto e a auditoria.".into() } else { format!("ATENÇÃO:\n- {}", c.problems.join("\n- ")) };
                }
                2 => {
                    app.desk.laudo_msg = pericia_core::report::preview(app.sim.as_ref().unwrap());
                }
                3 => {
                    app.desk.laudo_msg = match crate::save::export(app.sim.as_ref().unwrap()) {
                        Ok((p, m)) => format!("Exportado:\n{}\n{}", p.display(), m.display()),
                        Err(e) => format!("ERRO ao exportar: {}", e),
                    };
                }
                _ => {
                    let out = app.run("finalizar");
                    app.finish_case(out);
                }
            }
        }
    }
    let mut y = body.y + bh + 6.0 * s;
    let kind = if doc == DocKind::Laudo { "LAUDO PERICIAL" } else { "TERMO DE APREENSÃO (o POP 5.05 não prevê laudo)" };
    app.ui.text(kind, body.x, y + 14.0 * s, 13.0, F::Bold, T.accent);
    if kw_needed {
        let (txt, col) = if claim.is_empty() { ("Expressão de interesse: não indicada (use 'achei <expressão>' no terminal)".to_string(), T.dim) } else { (format!("Expressão de interesse: «{}»", claim), T.highlight) };
        let w = app.ui.measure(kind, 13.0, F::Bold);
        app.ui.text(&txt, body.x + w + 16.0 * s, y + 14.0 * s, 12.5, F::Bold, col);
    }
    y += 22.0 * s;
    let cols = 2;
    let fields_area_h = body.h * 0.70 - (y - body.y);
    let fh = fields_area_h / 3.0;
    let fw = (body.w - 8.0 * s) / cols as f32;
    let n_fields = if doc == DocKind::Laudo { 6 } else { 3 };
    for i in 0..n_fields {
        let idx = if doc == DocKind::Laudo { i } else { [0, 3, 4][i] };
        let (_, label) = LAUDO_FIELDS[idx];
        let label = if doc != DocKind::Laudo && idx == 0 { "Descrição do material apreendido" } else if doc != DocKind::Laudo && idx == 4 { "Observações finais" } else { label };
        let cx = body.x + (i % cols) as f32 * (fw + 8.0 * s);
        let cy = y + (i / cols) as f32 * fh;
        app.ui.text(label, cx, cy + 12.0 * s, 11.5, F::Bold, T.dim);
        let r = rect(cx, cy + 16.0 * s, fw, fh - 22.0 * s);
        let id = format!("laudo{}", idx);
        let mut buf = std::mem::take(&mut app.desk.laudo[idx]);
        let mut sc = app.desk.laudo_scroll[idx];
        app.ui.text_area(&id, r, &mut buf, &mut sc);
        app.desk.laudo[idx] = buf;
        app.desk.laudo_scroll[idx] = sc;
    }
    let msg_r = rect(body.x, body.y + body.h * 0.72, body.w, body.h * 0.28);
    let t = if app.desk.laudo_msg.is_empty() {
        "Os campos são enviados ao motor como comandos 'laudo <campo> <texto>' (registrados na auditoria).\nSepare FATOS (o que foi observado) de INFERÊNCIAS (o que se conclui) e registre as limitações.".to_string()
    } else {
        app.desk.laudo_msg.clone()
    };
    let mut sc = app.desk.scroll;
    app.ui.scroll_text(msg_r, &t, &mut sc, 11.5, F::Mono, T.term_fg, false);
    app.desk.scroll = sc;
}

pub fn manual(app: &mut App, body: Rect) {
    let s = app.ui.scale;
    let bh = 28.0 * s;
    let bw = (body.w - 4.0 * 5.0 * s) / 6.0;
    for (i, m) in ModuleId::ALL.iter().enumerate() {
        if app.ui.tab(rect(body.x + i as f32 * (bw + 4.0 * s), body.y, bw, bh), &format!("POP {}", m.code()), app.desk.manual_tab == *m) {
            app.desk.manual_tab = *m;
            app.desk.scroll = 0.0;
        }
    }
    let txt = format!(
        "{}\n\n— Resumo para estudo. Fonte: POP Perícia Criminal 2024 — Informática Forense, vol. 5 (SENASP/MJSP). Trechos marcados [complemento] não vêm do POP.\n\nCOMANDOS DO SIMULADOR\n{}\n\n{}\n\n{}\n\n{}",
        pericia_core::manual::pop_summary(app.desk.manual_tab),
        pericia_core::tools::help(""),
        pericia_core::tools::help("fs"),
        pericia_core::tools::help("rede"),
        pericia_core::tools::help("laudo")
    );
    let r = rect(body.x, body.y + bh + 6.0 * s, body.w, body.h - bh - 6.0 * s);
    let mut sc = app.desk.scroll;
    app.ui.scroll_text(r, &txt, &mut sc, 12.0, F::Mono, T.text, false);
    app.desk.scroll = sc;
}
