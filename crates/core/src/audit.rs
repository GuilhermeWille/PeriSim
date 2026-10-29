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
        let payload = format!("{}|{}|{}|{}|{}|{}|{}", seq, sim_time, actor, action, detail, outcome, prev);
        let hash = sha256(&payload);
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
        for e in &self.entries {
            let payload = format!("{}|{}|{}|{}|{}|{}|{}", e.seq, e.sim_time, e.actor, e.action, e.detail, e.outcome, prev);
            if sha256(&payload) != e.hash || e.prev_hash != prev {
                return Err(e.seq);
            }
            prev = e.hash.clone();
        }
        Ok(())
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
    fn encadeamento_usa_hash_anterior() {
        let mut a = Audit::new();
        a.record(1, "p", "a", "d", "o");
        let h1 = a.entries[0].hash.clone();
        a.record(2, "p", "b", "d", "o");
        assert_eq!(a.entries[1].prev_hash, h1);
    }
}
