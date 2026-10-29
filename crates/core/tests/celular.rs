//! Tela do aparelho celular: o que é exibido em cada estado e as regras da navegação manual.

use pericia_core::casegen::generate;
use pericia_core::engine::Sim;
use pericia_core::model::*;
use pericia_core::phone::{view, Screen};

fn phone(s: &Sim) -> PhoneState {
    s.case.items[0].phone.clone().unwrap()
}

#[test]
fn d1_desbloqueado_mostra_tela_inicial_com_conteudo_do_caso() {
    for seed in 1..20 {
        let s = Sim::new(generate(ModuleId::M502, 1, seed));
        let v = view(&s, "ITEM-01").unwrap();
        assert_eq!(v.screen, Screen::Home, "semente {}", seed);
        assert!(!v.threads.is_empty(), "conversas");
        assert!(!v.calls.is_empty(), "chamadas");
        assert!(!v.photos.is_empty(), "galeria");
        assert!(!v.contacts.is_empty(), "contatos");
        // arquivos apagados não aparecem na tela
        assert!(v.files.iter().all(|f| !f.path.contains("comprovante")));
        // a conversa contém a expressão do caso
        let kw = s.case.keyword.to_lowercase();
        assert!(v.threads.iter().any(|t| t.msgs.iter().any(|m| m.text.to_lowercase().contains(&kw))));
    }
}

#[test]
fn bloqueado_nao_expoe_conteudo_pela_tela() {
    let mut achou = false;
    for seed in 1..40 {
        let mut s = Sim::new(generate(ModuleId::M502, 3, seed));
        if phone(&s).boot == Boot::Desbloqueado {
            continue;
        }
        achou = true;
        let v = view(&s, "ITEM-01").unwrap();
        assert!(matches!(v.screen, Screen::Locked { .. }));
        let o = s.exec("tela item-01 mensagens");
        assert!(!o.ok);
        assert!(!s.keyword_found);
        assert!(!s.audit.entries.iter().any(|e| e.action == "tela"));
    }
    assert!(achou);
}

#[test]
fn desligado_mostra_tela_apagada() {
    let mut s = Sim::new(generate(ModuleId::M502, 1, 3));
    s.exec("isolar item-01 faraday");
    s.exec("energia desligar item-01");
    assert_eq!(view(&s, "ITEM-01").unwrap().screen, Screen::Off);
    assert!(!s.exec("tela item-01 config").ok);
}

#[test]
fn navegar_antes_da_extracao_e_falta_e_marca_mensagens_como_lidas() {
    let mut s = Sim::new(generate(ModuleId::M502, 1, 7));
    let antes = view(&s, "ITEM-01").unwrap().notifications;
    assert!(antes > 0);
    let o = s.exec("tela item-01 mensagens");
    assert!(o.ok, "{}", o.text);
    assert!(s.faults.iter().any(|f| f.contains("antes da extração")));
    assert_eq!(view(&s, "ITEM-01").unwrap().notifications, 0);
    // a expressão vista na tela é registrada com a origem
    assert!(s.keyword_found);
    assert!(s.keyword_where.contains("mensagens"));
    // uma única falta, mesmo navegando de novo
    s.exec("tela item-01 galeria");
    assert_eq!(s.faults.iter().filter(|f| f.contains("antes da extração")).count(), 1);
}

#[test]
fn navegar_depois_da_extracao_e_complemento_documentado() {
    let mut s = Sim::new(generate(ModuleId::M502, 1, 7));
    s.exec("isolar item-01 faraday");
    s.exec("carregar item-01");
    assert!(s.exec("extrair item-01").ok);
    let o = s.exec("tela item-01 mensagens");
    assert!(o.ok);
    assert!(!s.faults.iter().any(|f| f.contains("antes da extração")), "{:?}", s.faults);
    assert!(s.exec("tela item-01 foto").ok);
    assert!(s.audit.has("tela-foto"));
}

#[test]
fn apreensao_permite_configuracoes_mas_nao_conteudo() {
    let mut achou = false;
    for seed in 1..40 {
        let mut s = Sim::new(generate(ModuleId::M505, 1, seed));
        if view(&s, "ITEM-01").unwrap().screen != Screen::Home {
            continue;
        }
        achou = true;
        assert!(s.exec("tela item-01 config").ok);
        assert!(s.exec("tela item-01 relogio").ok);
        assert!(s.faults.is_empty(), "{:?}", s.faults);
        s.exec("tela item-01 galeria");
        assert!(s.faults.iter().any(|f| f.contains("apreensão")));
    }
    assert!(achou);
}

#[test]
fn imei_pela_tela_usa_o_codigo_de_servico() {
    let mut s = Sim::new(generate(ModuleId::M502, 2, 4));
    let o = s.exec("tela item-01 imei");
    assert!(o.ok, "{}", o.text);
    assert!(s.audit.has("imei-verificar"));
}

#[test]
fn restaurado_mostra_assistente_de_configuracao() {
    let mut s = Sim::new(generate(ModuleId::M502, 1, 2));
    s.case.items[0].phone.as_mut().unwrap().wiped = true;
    assert_eq!(view(&s, "ITEM-01").unwrap().screen, Screen::Setup);
}

#[test]
fn modo_aviao_reflete_na_barra_de_status() {
    let mut s = Sim::new(generate(ModuleId::M502, 1, 9));
    assert!(s.exec("airplane item-01").ok);
    let v = view(&s, "ITEM-01").unwrap();
    assert!(v.airplane && !v.wifi && !v.cellular);
    assert_eq!(v.operator, "Modo avião");
}
