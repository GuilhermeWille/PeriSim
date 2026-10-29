//! Conferência da geração procedural (gerador atual, 1.2.0): invariantes de todos os
//! módulos e níveis em muitas sementes, e um "perito automático" que prova que cada caso
//! pode ser cumprido no modo passo a passo (ferramentas, etapas físicas e aquisição).

use pericia_core::casegen::{case_id, generate, parse_case_id, GEN_VERSION};
use pericia_core::engine::Sim;
use pericia_core::equip::{cover_screws, media_connector, phone_cable, slot_has_cables, slot_screws, tool_id};
use pericia_core::model::*;
use std::collections::BTreeSet;

const SEEDS: u32 = 120;

#[test]
fn invariantes_de_todos_os_casos() {
    let mut casos = 0;
    let mut com_vestigio = 0;
    for m in ModuleId::ALL {
        for d in 1..=4u8 {
            for seed in 0..SEEDS {
                let c = generate(m, d, seed);
                casos += 1;
                let tag = format!("{}", case_id(m, d, seed));
                assert_eq!(c.gen_version, GEN_VERSION, "{}", tag);
                assert_eq!(parse_case_id(&c.id), Some((m, d, seed)), "{}", tag);
                // determinismo: gerar de novo produz exatamente o mesmo caso
                let a = serde_json::to_string(&c).unwrap();
                assert_eq!(a, serde_json::to_string(&generate(m, d, seed)).unwrap(), "{} não determinístico", tag);
                // identificadores únicos
                let ids: BTreeSet<&str> = c.items.iter().map(|i| i.id.as_str()).collect();
                assert_eq!(ids.len(), c.items.len(), "{} itens repetidos", tag);
                let mids: BTreeSet<&str> = c.media.iter().map(|x| x.id.as_str()).collect();
                assert_eq!(mids.len(), c.media.len(), "{} mídias repetidas", tag);
                // referências coerentes
                for it in &c.items {
                    for mid in &it.media {
                        assert!(mids.contains(mid.as_str()), "{} {} cita mídia inexistente {}", tag, it.id, mid);
                    }
                    for sl in &it.slots {
                        if let Some(mid) = &sl.media_id {
                            assert!(it.media.contains(mid), "{} slot {} com mídia fora do item", tag, sl.id);
                        }
                        if let Some(h) = &sl.hidden_by {
                            assert!(it.components.iter().any(|x| &x.kind == h), "{} slot oculto por componente inexistente", tag);
                        }
                    }
                    if it.kind.is_computer() && !it.slots.is_empty() && !it.power_on {
                        assert!(cover_screws(it) > 0 || it.kind == ItemKind::Notebook, "{}", tag);
                    }
                }
                for (mid, h) in &c.documented_hashes {
                    let md = c.media_by_id(mid).unwrap();
                    assert_eq!(&pericia_core::vfs::media_hash(md), h, "{} hash documentado de {} não confere", tag, mid);
                }
                assert!(!c.objetivo.is_empty() && !c.historico.is_empty(), "{} sem objetivo/histórico", tag);
                // a apreensão (5.05) gera termo, não laudo: não tem quesitos
                assert!(m == ModuleId::M505 || !c.quesitos.is_empty(), "{} sem quesitos", tag);
                // vestígios: ids, âncoras e itens válidos; biológico marca o item
                let vids: BTreeSet<&str> = c.vestigios.iter().map(|v| v.id.as_str()).collect();
                assert_eq!(vids.len(), c.vestigios.len(), "{}", tag);
                if !c.vestigios.is_empty() {
                    com_vestigio += 1;
                }
                for v in &c.vestigios {
                    assert!(["item", "piso_mesa", "teclado", "mesa", "bancada", "cinzeiro"].contains(&v.spot.as_str()), "{} âncora {}", tag, v.spot);
                    if let Some(on) = &v.on_item {
                        let it = c.item(on).expect("vestígio em item inexistente");
                        assert!(!it.hidden, "{} vestígio num item oculto", tag);
                        if v.biologico {
                            assert!(it.physical.biohazard, "{}", tag);
                        }
                    } else {
                        assert_eq!(c.setting, Setting::Local, "{} vestígio de ambiente fora de local", tag);
                    }
                }
            }
        }
    }
    assert!(com_vestigio > casos / 4, "vestígios aparecem em boa parte dos casos ({} de {})", com_vestigio, casos);
}

/// Executa e devolve se deu certo (comandos que não se aplicam ao caso podem falhar).
fn x(s: &mut Sim, c: &str) -> bool {
    s.exec(c).ok
}

/// Perito automático do modo passo a passo: segue a ordem física para todo o material.
fn perito(s: &mut Sim) {
    x(s, "luvas calcar");
    x(s, "pulseira colocar");
    x(s, "pegar lanterna");
    x(s, "inspecionar local");
    for v in s.case.vestigios.clone() {
        let id = v.id.to_lowercase();
        x(s, "pegar placas");
        assert!(x(s, &format!("marcar {}", id)));
        x(s, "pegar camera");
        assert!(x(s, &format!("fotografar {}", id)));
        assert!(x(s, &format!("comunicar {}", id)));
    }
    let itens: Vec<Item> = s.case.items.iter().filter(|i| !i.hidden || i.flag("descoberto")).cloned().collect();
    for it0 in itens {
        let id = it0.id.to_lowercase();
        if s.case.items.iter().any(|i| i.physical.biohazard) {
            x(s, "luvas trocar"); // luvas novas a cada objeto quando há material biológico
        }
        x(s, "pegar camera");
        assert!(x(s, &format!("fotografar {}", id)));
        if it0.seal.is_some() && it0.doc_seal.is_some() {
            x(s, "pegar oficio");
            assert!(x(s, &format!("lacre conferir {}", id)));
        }
        if it0.seal.as_ref().map(|x| x.intact).unwrap_or(false) {
            x(s, "pegar tesoura");
            assert!(x(s, &format!("deslacrar {}", id)));
        }
        if it0.kind.is_computer() {
            if it0.power_on {
                x(s, "pegar pendrive-coleta");
                x(s, &format!("memoria {}", id));
                x(s, &format!("volume copiar {}", id));
                x(s, &format!("rede isolar {}", id));
                assert!(x(s, &format!("energia desligar {}", id)), "desligar {}", id);
            }
            let it = s.case.item(&it0.id).unwrap().clone();
            x(s, "pegar chave");
            for _ in 0..cover_screws(&it) {
                assert!(x(s, &format!("desparafusar {}", id)));
            }
            if cover_screws(&it) > 0 {
                assert!(x(s, &format!("abrir {}", id)), "abrir {}", id);
            }
            for c in it.components.iter().filter(|c| c.kind != "tampa") {
                if it.slots.iter().any(|sl| sl.hidden_by.as_deref() == Some(c.kind.as_str())) {
                    assert!(x(s, &format!("componente remover {} {}", id, c.id)));
                }
            }
            let it = s.case.item(&it0.id).unwrap().clone();
            for sl in it.slots.iter().filter(|sl| sl.media_id.is_some()) {
                let slot = sl.id.to_lowercase();
                if slot_has_cables(&it, sl) {
                    assert!(x(s, &format!("cabo desconectar {} {} dados", id, slot)));
                    assert!(x(s, &format!("cabo desconectar {} {} energia", id, slot)));
                }
                for _ in 0..slot_screws(&s.case, &it, sl) {
                    assert!(x(s, &format!("desparafusar {} {}", id, slot)));
                }
                let o = s.exec(&format!("midia remover {} {}", id, slot));
                assert!(o.ok, "{}: remover {} — {}", s.case.id, slot, o.text);
            }
        }
        if let Some(ph) = it0.phone.clone() {
            x(s, "pegar faraday");
            x(s, &format!("isolar {} faraday", id));
            x(s, "pegar bateria");
            x(s, &format!("carregar {}", id));
            if s.case.module != ModuleId::M505 {
                x(s, &format!("pegar {}", tool_id(phone_cable(&ph))));
                let o = s.exec(&format!("extrair {}", id));
                assert!(!o.text.contains("segure"), "{}: {}", s.case.id, o.text);
            }
        }
    }
    // aquisição de toda mídia removível (exceto a cifrada por hardware, que tem rota própria)
    let mut usb = false;
    for md in s.case.media.clone() {
        if md.kind == MediaKind::FlashMovel || md.fde.is_some() || md.is_work_media {
            continue;
        }
        let host_visible = s.case.items.iter().any(|i| i.media.contains(&md.id) && (!i.hidden || i.flag("descoberto")));
        if !host_visible {
            continue;
        }
        let mid = md.id.to_lowercase();
        if let Some(t) = media_connector(&md) {
            x(s, &format!("pegar {}", tool_id(t)));
        }
        let o = s.exec(&format!("conectar {} bloqueador", mid));
        assert!(o.ok, "{}: conectar {} — {}", s.case.id, mid, o.text);
        if !usb {
            x(s, "pegar cabo-usb");
            assert!(x(s, "conectar bloqueador estacao"));
            usb = true;
        }
        assert!(x(s, &format!("bloqueio {}", mid)));
        let o = s.exec(&format!("imagem {}", mid));
        assert!(o.ok, "{}: imagem {} — {}", s.case.id, mid, o.text);
    }
}

#[test]
fn todo_caso_e_cumprivel_passo_a_passo() {
    for m in ModuleId::ALL {
        for d in 1..=4u8 {
            for seed in 0..40u32 {
                let mut s = Sim::new(generate(m, d, seed));
                perito(&mut s);
                let ruins: Vec<&String> = s.faults.iter().filter(|f| f.contains("sem luvas") || f.contains("pulseira") || f.contains("contaminação cruzada") || f.contains("sem bloqueador")).collect();
                assert!(ruins.is_empty(), "{}: {:?}", s.case.id, ruins);
                assert!(s.audit.verify().is_ok());
                let cl = pericia_core::assess::checklist(&s);
                for (ok, l) in &cl {
                    if l.contains("luvas") || l.contains("Vestígios") || l.contains("pulseira") {
                        assert!(ok, "{}: etapa '{}' não cumprida", s.case.id, l);
                    }
                }
            }
        }
    }
}
