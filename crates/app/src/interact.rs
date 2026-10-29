//! Interação física na cena 3D: pegar e largar ferramentas da bancada, usar a ferramenta da
//! mão na próxima etapa do objeto mirado (na ordem do procedimento), cabos conectados ponta a
//! ponta, indicadores de onde a ferramenta serve, vista de exame (zoom girando em volta do
//! objeto) e o objetivo marcado no tablet.

use crate::steps::{self, Alvo, Step, Uso};
use crate::ui::*;
use crate::{bench, world, App};
use macroquad::prelude::*;
use pericia_core::engine::Sim;
use pericia_core::equip::{tool_name, Gloves, Tool};
use pericia_core::model::*;

/// Alcance de braço para interagir (mesmo da mira).
pub const REACH: f32 = 2.6;

/// Vista de exame: câmera orbitando o objeto real da cena.
#[derive(Clone)]
pub struct Exam {
    pub id: String,
    pub center: Vec3,
    pub half: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
}

/// Cabo com uma ponta já conectada, esperando a outra.
#[derive(Clone)]
pub struct Cabo {
    pub cmd: String,
    pub tool: Option<Tool>,
    pub feito: Alvo,
    pub falta: Alvo,
    pub ponto: Vec3,
}

/// Objetivo marcado no tablet (mostrado discretamente na HUD).
#[derive(Clone)]
pub struct Objetivo {
    pub owner: String,
    pub cmd: String,
    pub label: String,
}

fn is_storage_item(it: &Item) -> bool {
    matches!(it.kind, ItemKind::Hdd | ItemKind::Ssd | ItemKind::Nvme | ItemKind::Pendrive | ItemKind::CartaoSd) && it.slots.is_empty()
}

/// O alvo da etapa corresponde ao objeto mirado?
pub fn alvo_hit(sim: &Sim, a: &Alvo, pick: &str) -> bool {
    match a {
        Alvo::Objeto(id) | Alvo::Vestigio(id) => pick == id,
        Alvo::Midia(mid) => pick.strip_prefix("M:") == Some(mid.as_str()) || sim.case.item(pick).map(|it| is_storage_item(it) && it.media.len() == 1 && &it.media[0] == mid).unwrap_or(false),
        Alvo::Bloqueador => pick == "TOOL-WB",
        Alvo::Estacao => pick == "STATION",
    }
}

pub fn alvo_desc(sim: &Sim, a: &Alvo) -> String {
    match a {
        Alvo::Objeto(id) | Alvo::Vestigio(id) => id.clone(),
        Alvo::Midia(mid) => {
            let ext = sim.case.items.iter().find(|it| is_storage_item(it) && it.media.first() == Some(mid));
            match ext {
                Some(it) => format!("{} ({})", mid, it.id),
                None => format!("mídia {}", mid),
            }
        }
        Alvo::Bloqueador => "bloqueador de escrita".into(),
        Alvo::Estacao => "estação pericial".into(),
    }
}

/// Etapas de todos os objetos e vestígios (calculadas uma vez por quadro).
pub fn all_steps(sim: &Sim) -> Vec<(String, Vec<Step>)> {
    steps::owners(sim).into_iter().map(|o| { let st = steps::steps_of(sim, &o); (o, st) }).collect()
}

fn tool_label(t: Option<Tool>) -> String {
    t.map(|t| tool_name(t).to_string()).unwrap_or_else(|| "mãos livres".into())
}

/// O que a ferramenta da mão faz no objeto mirado: (dono, resultado).
pub fn find_use(sim: &Sim, all: &[(String, Vec<Step>)], pick: &str, hand: Option<Tool>, obj: Option<&Objetivo>) -> Option<(String, Uso)> {
    let hit = |a: &Alvo| alvo_hit(sim, a, pick);
    let mut antes: Option<(String, Uso)> = None;
    let mut nada: Option<(String, Uso)> = None;
    for (owner, st) in all {
        if !st.iter().any(|s| s.alvos.iter().any(|a| hit(a))) {
            continue;
        }
        let marcado = obj.filter(|o| &o.owner == owner).map(|o| o.cmd.as_str());
        match steps::uso(st, &hit, hand, marcado) {
            u @ Uso::Fazer(_) => return Some((owner.clone(), u)),
            u @ Uso::Antes { .. } => {
                antes.get_or_insert((owner.clone(), u));
            }
            u @ Uso::Nada(_) => {
                nada.get_or_insert((owner.clone(), u));
            }
        }
    }
    antes.or(nada)
}

// ------------------------------------------------------------------ bancada

fn local(app: &App) -> bool {
    app.sim.as_ref().map(|s| s.case.setting == Setting::Local).unwrap_or(false)
}

/// E num item da bancada: luvas, pulseira ou pegar a ferramenta (a da mão fica no lugar dela).
pub fn bench_use(app: &mut App, id: &str) {
    let Some(sim) = app.sim.as_ref() else { return };
    let lc = local(app);
    match id {
        bench::LUVAS => {
            let c = match sim.equip.gloves {
                Gloves::Nenhuma => "luvas calcar",
                Gloves::Contaminadas(_) => "luvas trocar",
                Gloves::Limpas => {
                    app.toast("Luvas calçadas e limpas. Troque depois de tocar material biológico.");
                    return;
                }
            };
            let out = app.run(c);
            app.toast(out.lines().next().unwrap_or(""));
        }
        bench::PULSEIRA => {
            let c = if sim.equip.strap { "pulseira retirar" } else { "pulseira colocar" };
            let out = app.run(c);
            app.toast(out.lines().next().unwrap_or(""));
        }
        _ => {
            let Some(t) = bench::tool_of(id) else { return };
            let held = sim.equip.hand;
            if held == Some(t) {
                return;
            }
            // troca: a ferramenta da mão fica onde estava a que foi pega
            if let (Some(h), Some(spot)) = (held, app.bench.get(lc, id)) {
                if let Some(hid) = pericia_core::equip::TOOLS.iter().find(|x| x.0 == h).map(|x| x.1) {
                    app.bench.set(lc, hid, spot.0, spot.1);
                }
            }
            app.cabo = None;
            let out = app.run(&format!("pegar {}", id));
            if out.lines().next().map(|l| l.starts_with("Na mão")).unwrap_or(false) {
                app.toast(&format!("Na mão: {}", bench::name_of(id)));
            } else {
                app.toast(out.lines().next().unwrap_or(""));
            }
        }
    }
}

/// G: larga a ferramenta da mão no ponto mirado (bancada ou mesa).
pub fn put_down(app: &mut App, o: Vec3, d: Vec3, yaw: f32) {
    let Some(sim) = app.sim.as_ref() else { return };
    let Some(t) = sim.equip.hand else {
        app.toast("Mãos livres: nada para largar.");
        return;
    };
    let lc = local(app);
    let Some(p) = bench::place_ray(o, d, lc, REACH) else {
        app.toast("Mire num tampo de bancada ou mesa, ao alcance, para largar.");
        return;
    };
    let id = pericia_core::equip::TOOLS.iter().find(|x| x.0 == t).map(|x| x.1).unwrap_or("");
    // de frente para o perito
    app.bench.set(lc, id, p, yaw + std::f32::consts::PI);
    app.cabo = None;
    app.run("guardar");
    app.toast(&format!("{} largada na mesa.", tool_name(t)));
}

// ------------------------------------------------------------------ usar

/// E num objeto (item, vestígio, mídia, bloqueador, estação) com o que estiver na mão.
/// Devolve `false` quando não havia nada para fazer (quem chamou pode abrir o exame).
pub fn use_on(app: &mut App, pick: &str, ponto: Vec3) -> bool {
    let Some(sim) = app.sim.as_ref() else { return false };
    let hand = sim.equip.hand;
    // cabo com uma ponta conectada
    if let Some(c) = app.cabo.clone() {
        if c.tool != hand {
            app.cabo = None;
        } else if alvo_hit(sim, &c.falta, pick) {
            app.cabo = None;
            let out = app.run(&c.cmd);
            app.toast(out.lines().next().unwrap_or(""));
            return true;
        } else if alvo_hit(sim, &c.feito, pick) {
            app.toast(&format!("Esta ponta já está conectada. Leve a outra ponta até: {}.", alvo_desc(sim, &c.falta)));
            return true;
        } else {
            app.toast(&format!("Cabo com uma ponta em {}: conecte a outra em {} (G larga o cabo).", alvo_desc(sim, &c.feito), alvo_desc(sim, &c.falta)));
            return true;
        }
    }
    let all = all_steps(sim);
    let obj = app.objetivo.clone();
    let Some((_owner, u)) = find_use(sim, &all, pick, hand, obj.as_ref()) else { return false };
    match u {
        Uso::Fazer(st) => {
            if st.alvos.len() == 2 {
                let (feito, falta) = if alvo_hit(sim, &st.alvos[0], pick) { (st.alvos[0].clone(), st.alvos[1].clone()) } else { (st.alvos[1].clone(), st.alvos[0].clone()) };
                let msg = format!("Ponta conectada em {} — agora a outra ponta em {}.", alvo_desc(sim, &feito), alvo_desc(sim, &falta));
                app.cabo = Some(Cabo { cmd: steps::cmd_for_tool(&st, hand), tool: hand, feito, falta, ponto });
                app.audio.play("cabo_conectar", crate::audio::Cat::Efeitos, 0.6);
                app.toast(&msg);
            } else {
                let out = app.run(&steps::cmd_for_tool(&st, hand));
                app.toast(out.lines().next().unwrap_or(""));
            }
            true
        }
        Uso::Antes { quer, falta } => {
            let precisa = falta.tool.map(|t| format!(" — segure: {}", tool_name(t))).unwrap_or_default();
            app.toast(&format!("Antes de “{}”: falta “{}”{}.", quer.label, falta.label, precisa));
            true
        }
        Uso::Nada(nx) => {
            if hand.is_some() {
                let dica = nx.map(|n| format!(" Próxima etapa aqui: {} ({}).", n.label, tool_label(n.tool))).unwrap_or_default();
                app.toast(&format!("Nada a fazer com {} neste objeto.{}", tool_label(hand), dica));
                return true;
            }
            false
        }
    }
}

// ------------------------------------------------------------------ exame

pub fn start_exam(app: &mut App, id: &str, center: Vec3, half: Vec3, from: Vec3) {
    let d = (from - center).normalize_or_zero();
    let size = half.length().max(0.05);
    app.exam = Some(Exam { id: id.to_string(), center, half, yaw: d.x.atan2(d.z), pitch: d.y.asin().clamp(0.15, 1.2), dist: (size * 3.0).clamp(0.25, 1.6) });
    app.audio.play("papel", crate::audio::Cat::Interface, 0.25);
}

pub fn exam_camera(ex: &Exam, view: Rect, fov: f32) -> Camera3D {
    let dir = vec3(ex.yaw.sin() * ex.pitch.cos(), ex.pitch.sin(), ex.yaw.cos() * ex.pitch.cos());
    let pos = ex.center + dir * ex.dist;
    Camera3D { position: pos, target: ex.center, up: vec3(0.0, 1.0, 0.0), fovy: fov, aspect: Some(view.w / view.h), z_near: 0.01, z_far: 900.0, ..Default::default() }
}

/// Atualiza o exame (mouse gira, roda aproxima). Devolve `false` quando o exame termina.
pub fn exam_input(app: &mut App) -> bool {
    let Some(ex) = app.exam.as_mut() else { return false };
    let d = mouse_delta_position();
    ex.yaw += d.x * 3.0 * app.settings.mouse_sens;
    ex.pitch = (ex.pitch - d.y * 2.2 * app.settings.mouse_sens).clamp(-0.2, 1.45);
    let size = ex.half.length().max(0.05);
    if app.ui.wheel != 0.0 {
        ex.dist = (ex.dist * if app.ui.wheel > 0.0 { 0.88 } else { 1.12 }).clamp(size * 1.4, size * 6.0 + 0.3);
        app.ui.wheel = 0.0;
    }
    let id = ex.id.clone();
    let fk = matches!(app.fake_key, Some(KeyCode::F) | Some(KeyCode::E) | Some(KeyCode::Escape));
    if fk {
        app.fake_key = None;
    }
    if fk || app.ui.key_esc || is_key_pressed(KeyCode::F) || is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::Q) {
        app.ui.key_esc = false;
        app.exam = None;
        return false;
    }
    if app.ui.key_enter && app.sim.as_ref().and_then(|s| s.case.item(&id)).map(|it| it.phone.is_some()).unwrap_or(false) {
        app.exam = None;
        app.phone.close();
        app.phone.open = Some(id);
        return false;
    }
    true
}

/// O que se lê no objeto examinado: etiquetas, número de série, lacre, estado.
pub fn exam_info(sim: &Sim, id: &str) -> (String, Vec<(String, Color)>) {
    let mut v: Vec<(String, Color)> = vec![];
    let dim = T.dim;
    if let Some(mid) = id.strip_prefix("M:") {
        if let Some(m) = sim.case.media_by_id(mid) {
            v.push((format!("Etiqueta: {} {}", m.brand, m.model), T.text));
            v.push((format!("Nº de série: {}", m.serial), T.text));
            v.push((format!("Capacidade: {} · interface {}", pericia_core::util::fmt_bytes(m.capacity), m.iface), T.text));
        }
        return (format!("Mídia {} (retirada)", mid), v);
    }
    if id == "TOOL-WB" {
        let on = sim.hashes.keys().any(|k| k.starts_with("wb:"));
        v.push(("Portas: SATA, M.2 (adaptador), leitor SD · saída USB para a estação".into(), T.text));
        v.push((format!("LED de proteção contra escrita: {}", if on { "aceso (bloqueio ativo)" } else { "apagado" }), if on { T.ok } else { dim }));
        return ("Bloqueador de escrita".into(), v);
    }
    if let Some(vs) = sim.case.vestigios.iter().find(|x| x.id == id) {
        v.push((vs.desc.clone(), T.text));
        if vs.biologico {
            v.push(("Material biológico: não tocar sem luvas, não limpar, não coletar.".into(), T.warn));
        }
        return (format!("{} — {}", vs.id, vs.label), v);
    }
    let Some(it) = sim.case.item(id) else { return (id.to_string(), v) };
    if !it.brand.is_empty() || !it.model.is_empty() {
        v.push((format!("Etiqueta do fabricante: {} {}", it.brand, it.model), T.text));
    }
    if !it.serial.is_empty() {
        v.push((format!("Nº de série (S/N): {}", it.serial), T.text));
    }
    match &it.seal {
        Some(s) => v.push((format!("Lacre nº {} — {}", s.number, if s.intact { "íntegro" } else { "rompido" }), if s.intact { T.ok } else { T.warn })),
        None if it.doc_seal.is_some() => v.push(("Sem lacre físico (consta lacre no ofício)".into(), T.warn)),
        None => {}
    }
    if it.kind.is_computer() {
        v.push((format!("Energia: {}", if it.power_on { "LED aceso — ligado" } else { "desligado" }), if it.power_on { T.warn } else { dim }));
        if it.flag("aberto") {
            for sl in &it.slots {
                if let Some(m) = sl.media_id.as_ref().and_then(|m| sim.case.media_by_id(m)) {
                    if sl.hidden_by.is_none() && !sim.equip.removed.contains(&m.id) {
                        v.push((format!("{}: {} {} · S/N {} · {}", sl.id, m.brand, m.model, m.serial, pericia_core::util::fmt_bytes(m.capacity)), T.text));
                    }
                }
            }
        } else {
            v.push(("Tampa fechada: o interior não está visível.".into(), dim));
        }
    }
    if is_storage_item(it) {
        if let Some(m) = it.media.first().and_then(|m| sim.case.media_by_id(m)) {
            v.push((format!("Etiqueta da mídia: {} {} · S/N {}", m.brand, m.model, m.serial), T.text));
            v.push((format!("Capacidade impressa: {}", pericia_core::util::fmt_bytes(m.capacity)), T.text));
        }
    }
    if let Some(ph) = &it.phone {
        if !ph.imei_label.is_empty() {
            v.push((format!("IMEI na etiqueta: {}", ph.imei_label.join(" / ")), T.text));
        }
        let bat = if ph.battery <= 1.0 { ph.battery * 100.0 } else { ph.battery };
        v.push((format!("Bloqueio de tela: {} · bateria {:.0}%", if ph.lock.is_empty() { "—".to_string() } else { ph.lock.to_uppercase() }, bat), T.text));
        v.push((format!("Isolamento: {}", ph.rf.label()), if ph.rf.isolates() { T.ok } else { T.warn }));
    }
    (format!("{} — {}", it.id, it.label), v)
}

pub fn draw_exam(app: &mut App, view: Rect) {
    let Some(ex) = app.exam.clone() else { return };
    let Some(sim) = app.sim.as_ref() else { return };
    let s = app.ui.scale;
    let (title, lines) = exam_info(sim, &ex.id);
    let all = all_steps(sim);
    let next = all.iter().find(|(o, st)| o == &ex.id || st.iter().any(|x| x.alvos.iter().any(|a| alvo_hit(sim, a, &ex.id)))).and_then(|(_, st)| steps::next_required(st).cloned());
    let is_phone = sim.case.item(&ex.id).map(|it| it.phone.is_some()).unwrap_or(false);
    // vinheta forte: o foco é o objeto
    for k in 0..14 {
        let t = k as f32 / 14.0;
        let a = (1.0 - t) * 0.05;
        let inset = t * 160.0 * s;
        let c = Color::new(0.0, 0.0, 0.0, a);
        draw_rectangle(view.x, view.y + inset, view.w, 12.0 * s, c);
        draw_rectangle(view.x, view.y + view.h - inset - 12.0 * s, view.w, 12.0 * s, c);
        draw_rectangle(view.x + inset, view.y, 12.0 * s, view.h, c);
        draw_rectangle(view.x + view.w - inset - 12.0 * s, view.y, 12.0 * s, view.h, c);
    }
    let w = (360.0 * s).min(view.w * 0.42);
    let lw = w - 24.0 * s;
    let mut wrapped: Vec<(String, Color)> = vec![];
    for (l, c) in &lines {
        for x in app.ui.wrap(l, 12.0, F::Reg, lw) {
            wrapped.push((x, *c));
        }
    }
    let h = 44.0 * s + wrapped.len() as f32 * 17.0 * s + 64.0 * s;
    let r = rect(view.x + 16.0 * s, view.y + view.h - h - 16.0 * s, w, h);
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.03, 0.04, 0.06, 0.82));
    draw_rectangle(r.x, r.y, 3.0 * s, r.h, T.highlight);
    let t = crate::phone::clip_text(&app.ui, &title, 14.0, F::Bold, lw);
    app.ui.text(&t, r.x + 12.0 * s, r.y + 24.0 * s, 14.0, F::Bold, T.text);
    let mut y = r.y + 46.0 * s;
    for (l, c) in &wrapped {
        app.ui.text(l, r.x + 12.0 * s, y, 12.0, F::Reg, *c);
        y += 17.0 * s;
    }
    y += 6.0 * s;
    let nx = match &next {
        Some(n) => format!("Próxima etapa: {} — {}", n.label, tool_label(n.tool)),
        None => "Etapas obrigatórias concluídas.".into(),
    };
    let nx = crate::phone::clip_text(&app.ui, &nx, 11.5, F::Bold, lw);
    app.ui.text(&nx, r.x + 12.0 * s, y, 11.5, F::Bold, T.highlight);
    y += 18.0 * s;
    let ctl = if is_phone { "Mouse gira · roda aproxima · Enter: ver a tela · F ou Esc: voltar" } else { "Mouse gira · roda aproxima · F ou Esc: voltar" };
    app.ui.text(ctl, r.x + 12.0 * s, y, 10.5, F::Reg, T.dim);
}

// ------------------------------------------------------------------ indicadores e objetivo

/// Marcador discreto (losango) sobre um alvo, em coordenadas de tela.
fn marker(p: Vec2, s: f32, color: Color) {
    let r = 6.0 * s;
    let pts = [vec2(p.x, p.y - r), vec2(p.x + r, p.y), vec2(p.x, p.y + r), vec2(p.x - r, p.y)];
    let rr = r + 1.6;
    let out = [vec2(p.x, p.y - rr), vec2(p.x + rr, p.y), vec2(p.x, p.y + rr), vec2(p.x - rr, p.y)];
    draw_triangle(out[0], out[1], out[2], Color::new(0.0, 0.0, 0.0, 0.6));
    draw_triangle(out[0], out[2], out[3], Color::new(0.0, 0.0, 0.0, 0.6));
    draw_triangle(pts[0], pts[1], pts[2], color);
    draw_triangle(pts[0], pts[2], pts[3], color);
}

/// Com ferramenta na mão: marca cada componente onde ela pode ser usada agora. Com um cabo
/// pela metade: só a outra ponta. Mãos livres e objetivo marcado: o alvo e a ferramenta dele.
pub fn indicators(app: &App, cam: &Camera3D, view: Rect, picks: &[world::Pick], all: &[(String, Vec<Step>)]) {
    let Some(sim) = app.sim.as_ref() else { return };
    let s = app.ui.scale;
    let hand = sim.equip.hand;
    let pulse = 0.75 + 0.25 * ((get_time() * 4.0).sin() as f32);
    let draw_at = |p: &world::Pick, color: Color| {
        if (p.center - cam.position).length() > 7.0 {
            return;
        }
        if let Some(sp) = world::project(cam, view, p.center + vec3(0.0, p.half.y + 0.06, 0.0)) {
            if view.contains(sp) {
                marker(sp, s, color);
            }
        }
    };
    if let Some(c) = &app.cabo {
        for p in picks {
            if alvo_hit(sim, &c.falta, &p.id) {
                draw_at(p, Color::new(0.35, 0.95, 0.5, pulse));
            }
        }
        if let (Some(a), Some(b)) = (world::project(cam, view, c.ponto), Some(vec2(view.x + view.w * 0.78, view.y + view.h))) {
            draw_line(a.x, a.y, b.x, b.y, 3.0 * s, Color::new(0.12, 0.12, 0.14, 0.9));
        }
        return;
    }
    if hand.is_some() {
        let obj = app.objetivo.clone();
        for p in picks {
            if p.id.starts_with("T:") || p.id == "QUADRO" {
                continue;
            }
            if let Some((_, Uso::Fazer(st))) = find_use(sim, all, &p.id, hand, obj.as_ref()) {
                if st.tool.is_some() {
                    draw_at(p, Color::new(1.0, 0.82, 0.2, pulse));
                }
            }
        }
        return;
    }
    // mãos livres: o objetivo marcado aponta o alvo e a ferramenta que falta
    if let Some(o) = &app.objetivo {
        if let Some(st) = all.iter().find(|(ow, _)| ow == &o.owner).and_then(|(_, st)| st.iter().find(|x| x.cmd == o.cmd)) {
            for p in picks {
                if st.alvos.iter().any(|a| alvo_hit(sim, a, &p.id)) {
                    draw_at(p, Color::new(0.45, 0.75, 1.0, pulse));
                }
                if let Some(t) = st.tool {
                    if p.id.strip_prefix("T:").and_then(bench::tool_of) == Some(t) {
                        draw_at(p, Color::new(0.45, 0.75, 1.0, pulse));
                    }
                }
            }
        }
    }
}

/// Atualiza o objetivo marcado: quando a etapa termina, passa para a próxima do mesmo objeto.
pub fn advance_objective(app: &mut App, all: &[(String, Vec<Step>)]) {
    let Some(o) = app.objetivo.clone() else { return };
    let Some((_, st)) = all.iter().find(|(ow, _)| ow == &o.owner) else {
        app.objetivo = None;
        return;
    };
    let feito = st.iter().find(|x| x.cmd == o.cmd).map(|x| x.done).unwrap_or(true);
    if feito {
        app.objetivo = steps::next(st).map(|n| Objetivo { owner: o.owner.clone(), cmd: n.cmd.clone(), label: n.label.clone() });
        if app.objetivo.is_none() {
            app.toast(&format!("Objetivo concluído: todas as etapas de {}.", o.owner));
        }
    } else if let Some(x) = st.iter().find(|x| x.cmd == o.cmd) {
        // o rótulo muda com o andamento (ex.: parafusos 1/4 → 2/4)
        if x.label != o.label {
            app.objetivo = Some(Objetivo { label: x.label.clone(), ..o });
        }
    }
}

/// Faixa discreta no canto superior esquerdo da cena com o objetivo marcado.
pub fn objective_hud(app: &mut App, view: Rect, all: &[(String, Vec<Step>)]) {
    let Some(o) = app.objetivo.clone() else { return };
    let s = app.ui.scale;
    let tool = all.iter().find(|(ow, _)| ow == &o.owner).and_then(|(_, st)| st.iter().find(|x| x.cmd == o.cmd)).and_then(|x| x.tool);
    let txt = format!("{} · {}{}", o.owner, o.label, tool.map(|t| format!(" — {}", tool_name(t))).unwrap_or_default());
    let txt = crate::phone::clip_text(&app.ui, &txt, 12.0, F::Reg, view.w * 0.45);
    let w = app.ui.measure(&txt, 12.0, F::Reg) + 96.0 * s;
    let r = rect(view.x + 12.0 * s, view.y + 10.0 * s, w, 24.0 * s);
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.02, 0.03, 0.05, 0.55));
    draw_rectangle(r.x, r.y, 3.0 * s, r.h, Color::new(0.45, 0.75, 1.0, 0.9));
    app.ui.text("OBJETIVO", r.x + 10.0 * s, r.y + 16.5 * s, 10.0, F::Bold, Color::new(0.55, 0.8, 1.0, 1.0));
    app.ui.text(&txt, r.x + 80.0 * s, r.y + 16.5 * s, 12.0, F::Reg, T.text);
}

/// Texto da dica de mira para um alvo.
pub fn hover_text(sim: &Sim, all: &[(String, Vec<Step>)], p: &world::Pick, near: bool, cabo: Option<&Cabo>, obj: Option<&Objetivo>) -> String {
    if !near {
        return format!("{}  — aproxime-se", p.label);
    }
    if let Some(id) = p.id.strip_prefix("T:") {
        return match id {
            bench::LUVAS => format!("[E] {}", match sim.equip.gloves { Gloves::Nenhuma => "calçar luvas", Gloves::Contaminadas(_) => "trocar as luvas", Gloves::Limpas => "luvas já calçadas" }),
            bench::PULSEIRA => format!("[E] {} a pulseira antiestática", if sim.equip.strap { "retirar" } else { "colocar e aterrar" }),
            _ => format!("[E] pegar · {}", p.label),
        };
    }
    match p.id.as_str() {
        "STATION" if sim.equip.hand.is_none() && cabo.is_none() => return "[E] sentar na estação pericial".into(),
        "QUADRO" => return "[E] quadro de provas".into(),
        _ => {}
    }
    if let Some(c) = cabo {
        if alvo_hit(sim, &c.falta, &p.id) {
            return format!("[E] conectar a outra ponta · {}", p.label);
        }
    }
    let hand = sim.equip.hand;
    let what = match find_use(sim, all, &p.id, hand, obj) {
        Some((_, Uso::Fazer(st))) => format!("[E] {}", st.label.to_lowercase()),
        Some((_, Uso::Antes { falta, .. })) => format!("antes: {}", falta.label.to_lowercase()),
        Some((_, Uso::Nada(Some(n)))) => format!("próxima: {} ({})", n.label.to_lowercase(), tool_label(n.tool)),
        _ => String::new(),
    };
    let exam = if p.id == "STATION" { "" } else { "  [F] examinar" };
    if what.is_empty() {
        format!("{}{}", p.label, exam)
    } else {
        format!("{} — {}{}", p.label, what, exam)
    }
}
