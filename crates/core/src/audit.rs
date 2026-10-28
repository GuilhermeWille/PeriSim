//! Trilha de auditoria com encadeamento de hash (cadeia de custódia das AÇÕES).
//! Cada entrada inclui o hash da anterior, de modo que qualquer alteração retroativa
//! é detectável. É a base para verificar a coerência do laudo com o que foi feito.

use crate::sha256::sha256;
use crate::util::fmt_dtz;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub seq: u32,
    /// Instante simulado (segundos UTC) da ação.
    pub sim_time: i64,
    pub actor: String,
    pub action: String,
    pub detail: String,
    /// Resultado resumido (sucesso, falha, limitação).
    pub outcome: String,
    pub prev_hash: String,
    pub hash: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Audit {
    pub entries: Vec<AuditEntry>,
}

impl Audit {
    pub fn new() -> Self {
        Audit { entries: vec![] }
    }
    pub fn last_hash(&self) -> String {
        self.entries.last().map(|e| e.hash.clone()).unwrap_or_else(|| "GENESIS".to_string())
    }
    pub fn record(&mut self, sim_time: i64, actor: &str, action: &str, detail: &str, outcome: &str) -> &AuditEntry {
        let seq = self.entries.len() as u32 + 1;
        let prev = self.last_hash();
        let hash = sha256(&payload(seq, sim_time, actor, action, detail, outcome, &prev));
        self.entries.push(AuditEntry {
            seq,
            sim_time,
            actor: actor.to_string(),
            action: action.to_string(),
            detail: detail.to_string(),
            outcome: outcome.to_string(),
            prev_hash: prev,
            hash,
        });
        self.entries.last().unwrap()
    }
    /// Reconstrói a cadeia e aponta a primeira entrada adulterada, se houver.
    pub fn verify(&self) -> Result<(), u32> {
        let mut prev = "GENESIS".to_string();
        for (k, e) in self.entries.iter().enumerate() {
            if e.seq != k as u32 + 1 || e.prev_hash != prev || sha256(&payload(e.seq, e.sim_time, &e.actor, &e.action, &e.detail, &e.outcome, &prev)) != e.hash {
                return Err(e.seq.max(k as u32 + 1));
            }
            prev = e.hash.clone();
        }
        Ok(())
    }

    /// Último hash da cadeia no formato 1 (campos separados por '|', usado até a v0.3.1).
    /// Serve apenas para conferir salvamentos antigos; o formato 1 era ambíguo quando um
    /// campo continha '|' (ex.: linhas do shell com pipes).
    pub fn last_hash_v1(&self) -> String {
        let mut prev = "GENESIS".to_string();
        for e in &self.entries {
            prev = sha256(&format!("{}|{}|{}|{}|{}|{}|{}", e.seq, e.sim_time, e.actor, e.action, e.detail, e.outcome, prev));
        }
        prev
    }
    pub fn has(&self, action: &str) -> bool {
        self.entries.iter().any(|e| e.action == action)
    }
    pub fn count(&self, action: &str) -> usize {
        self.entries.iter().filter(|e| e.action == action).count()
    }
    pub fn find_last(&self, action: &str) -> Option<&AuditEntry> {
        self.entries.iter().rev().find(|e| e.action == action)
    }
    pub fn render(&self, offset: i32) -> String {
        let mut out = String::new();
        for e in &self.entries {
            out.push_str(&format!(
                "#{:03} [{}] {} — {}\n      {} | {}\n      hash {}…\n",
                e.seq,
                fmt_dtz(e.sim_time, offset),
                e.action,
                e.detail,
                e.actor,
                e.outcome,
                &e.hash[..12]
            ));
        }
        out
    }
}

/// Formato de cadeia vigente (gravado nos salvamentos).
pub const AUDIT_FORMAT: u32 = 2;

/// Conteúdo assinado de uma entrada (formato 2): cada campo é prefixado pelo seu tamanho
/// em bytes, de modo que nenhum conteúdo de campo pode imitar a fronteira entre campos.
fn payload(seq: u32, t: i64, actor: &str, action: &str, detail: &str, outcome: &str, prev: &str) -> String {
    let mut s = String::from("v2");
    for f in [seq.to_string().as_str(), t.to_string().as_str(), actor, action, detail, outcome, prev] {
        s.push_str(&format!("|{}:{}", f.len(), f));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cadeia_detecta_adulteracao() {
        let mut a = Audit::new();
        a.record(100, "perito", "conferir-lacre", "lacre 123", "íntegro");
        a.record(160, "perito", "deslacrar", "item 01", "ok");
        a.record(220, "perito", "hash", "M-01", "abc");
        assert!(a.verify().is_ok());
        // adultera o detalhe da 2ª entrada
        a.entries[1].detail = "item 99".into();
        assert_eq!(a.verify(), Err(2));
    }
    #[test]
    fn campos_com_separador_nao_colidem() {
        // no formato 1, ("a|b","c") e ("a","b|c") produziam o mesmo conteúdo assinado
        let mut x = Audit::new();
        x.record(1, "p", "sh", "ls | wc", "ok");
        let mut y = Audit::new();
        y.record(1, "p", "sh", "ls ", " wc|ok");
        assert_ne!(x.last_hash(), y.last_hash());
        assert_eq!(x.last_hash_v1(), y.last_hash_v1(), "demonstra a ambiguidade do formato antigo");
    }
    #[test]
    fn reordenar_ou_remover_entradas_e_detectado() {
        let mut a = Audit::new();
        for k in 0..4 {
            a.record(k, "p", "acao", &format!("d{}", k), "ok");
        }
        let mut b = a.clone();
        b.entries.remove(1);
        assert!(b.verify().is_err());
        let mut c = a.clone();
        c.entries.swap(1, 2);
        assert!(c.verify().is_err());
    }
    #[test]
    fn encadeamento_usa_hash_anterior() {
        let mut a = Audit::new();
        a.record(1, "p", "a", "d", "o");
        let h1 = a.entries[0].hash.clone();
        a.record(2, "p", "b", "d", "o");
        assert_eq!(a.entries[1].prev_hash, h1);
    }
}
