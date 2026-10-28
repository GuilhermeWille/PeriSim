//! Evidências em nuvem: autorização judicial, requisição ao provedor e captura direta.

use pericia_core::casegen::generate_classic as generate;
use pericia_core::engine::Sim;
use pericia_core::model::*;

fn caso(autorizado: bool) -> Sim {
    for seed in 1..200 {
        let c = generate(ModuleId::M501, 4, seed);
        if c.net.as_ref().and_then(|n| n.cloud.as_ref()).is_some() && c.legal.autorizacao_nuvem == autorizado {
            return Sim::new(c);
        }
    }
    panic!("nenhum caso com nuvem (autorizado={})", autorizado);
}

fn conta(s: &Sim) -> (String, String) {
    let c = s.case.net.as_ref().unwrap().cloud.as_ref().unwrap();
    (c.account.clone(), c.password.clone())
}

#[test]
fn sem_autorizacao_nao_ha_acesso_e_tentativa_direta_e_falta() {
    let mut s = caso(false);
    let (acc, pw) = conta(&s);
    assert!(!s.exec(&format!("nuvem solicitar {}", acc)).ok);
    assert!(s.faults.is_empty(), "solicitar sem autorização não é falta: {:?}", s.faults);
    let o = s.exec(&format!("nuvem acessar {} {}", acc, pw));
    assert!(!o.ok);
    assert!(s.faults.iter().any(|f| f.contains("sem autorização judicial")));
    assert!(!s.exec("nuvem ls").ok);
    pericia_core::report::autofill(&mut s);
    assert!(s.report.consideracoes.contains("não abrange a nuvem"), "{}", s.report.consideracoes);
}

#[test]
fn com_autorizacao_a_requisicao_ao_provedor_entrega_os_dados() {
    let mut s = caso(true);
    let (acc, _) = conta(&s);
    let t0 = s.clock;
    assert!(!s.exec("nuvem solicitar conta-errada@x.sim").ok);
    let o = s.exec(&format!("nuvem solicitar {}", acc));
    assert!(o.ok, "{}", o.text);
    assert!(s.clock - t0 >= 120 * 60, "o provedor leva tempo para responder");
    assert!(s.artifacts.iter().any(|a| a.kind == "nuvem-provedor"));
    let ls = s.exec("nuvem ls").text;
    let path = ls.lines().nth(1).unwrap().split_whitespace().last().unwrap().to_string();
    assert!(s.exec(&format!("nuvem cat {}", path)).ok);
    assert!(s.keyword_found, "a expressão estava no arquivo da nuvem");
    assert!(s.keyword_where.contains("provedor"));
    assert!(s.faults.is_empty(), "{:?}", s.faults);
}

#[test]
fn captura_direta_bloqueia_a_conta_apos_tres_erros() {
    let mut s = caso(true);
    s.exec("conexao dedicada");
    let (acc, pw) = conta(&s);
    for _ in 0..3 {
        assert!(!s.exec(&format!("nuvem acessar {} errada", acc)).ok);
    }
    assert!(s.cloud_locked);
    let o = s.exec(&format!("nuvem acessar {} {}", acc, pw));
    assert!(!o.ok && o.text.contains("bloqueada"));
    // a via judicial continua disponível
    assert!(s.exec(&format!("nuvem solicitar {}", acc)).ok);
}

#[test]
fn captura_direta_correta_registra_ip_e_limitacao() {
    let mut s = caso(true);
    s.exec("conexao dedicada");
    let (acc, pw) = conta(&s);
    let o = s.exec(&format!("nuvem acessar {} {}", acc, pw));
    assert!(o.ok, "{}", o.text);
    assert!(o.text.contains("IP usado"));
    assert!(s.faults.is_empty(), "{:?}", s.faults);
    pericia_core::report::autofill(&mut s);
    assert!(s.report.consideracoes.contains("captura direta"));
}

#[test]
fn captura_direta_por_conexao_institucional_e_falta() {
    let mut s = caso(true);
    let (acc, pw) = conta(&s);
    s.exec("conexao institucional");
    assert!(s.exec(&format!("nuvem acessar {} {}", acc, pw)).ok);
    assert!(s.faults.iter().any(|f| f.contains("conexão vinculada à instituição")));
}
