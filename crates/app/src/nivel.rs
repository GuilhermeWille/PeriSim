//! Nível do perito: a experiência real acumulada. Cada caso concluído com aprovação vale
//! tantos níveis quanto a sua dificuldade (1 a 4); o mesmo caso só conta uma vez. O nível só
//! dá títulos — de Novato a Expert —, nunca vantagens no jogo. Calculado do histórico de casos
//! concluídos (saves/historico.json), então vale também para casos de versões anteriores.

use crate::save::HistoryEntry;
use std::collections::BTreeMap;

/// (nível mínimo, título)
pub const TITULOS: [(u32, &str); 8] = [
    (0, "Novato"),
    (3, "Aprendiz"),
    (7, "Perito em formação"),
    (12, "Perito júnior"),
    (18, "Perito"),
    (26, "Perito sênior"),
    (36, "Especialista"),
    (50, "Expert"),
];

pub struct Perfil {
    pub nivel: u32,
    pub titulo: &'static str,
    /// Próximo título e o nível em que chega.
    pub proximo: Option<(&'static str, u32)>,
    /// Nível em que começou o título atual (para a barra de progresso).
    pub inicio: u32,
    pub casos: usize,
}

pub fn titulo(nivel: u32) -> &'static str {
    TITULOS.iter().rev().find(|(n, _)| nivel >= *n).map(|x| x.1).unwrap_or("Novato")
}

fn aprovado(e: &HistoryEntry) -> bool {
    e.verdict.starts_with("Aprovado")
}

/// Níveis por caso aprovado (um por ID, o de maior dificuldade registrada).
fn casos(h: &[HistoryEntry]) -> BTreeMap<String, u32> {
    let mut m = BTreeMap::new();
    for e in h.iter().filter(|e| aprovado(e)) {
        let d = e.difficulty.clamp(1, 4) as u32;
        let v = m.entry(e.id.to_uppercase()).or_insert(0);
        *v = (*v).max(d);
    }
    m
}

pub fn perfil(h: &[HistoryEntry]) -> Perfil {
    let c = casos(h);
    let nivel: u32 = c.values().sum();
    let i = TITULOS.iter().rposition(|(n, _)| nivel >= *n).unwrap_or(0);
    Perfil { nivel, titulo: TITULOS[i].1, proximo: TITULOS.get(i + 1).map(|x| (x.1, x.0)), inicio: TITULOS[i].0, casos: c.len() }
}

/// Níveis que este caso dá agora (0 se reprovado ou se já tinha sido aprovado antes).
pub fn ganho(antes: &[HistoryEntry], id: &str, dificuldade: u8, aprovado_agora: bool) -> u32 {
    if !aprovado_agora || casos(antes).contains_key(&id.to_uppercase()) {
        0
    } else {
        dificuldade.clamp(1, 4) as u32
    }
}

#[cfg(test)]
mod t {
    use super::*;

    fn e(id: &str, d: u8, v: &str) -> HistoryEntry {
        HistoryEntry { id: id.into(), module: "501".into(), difficulty: d, score: 80, max: 100, verdict: v.into(), when: String::new() }
    }

    #[test]
    fn niveis_por_dificuldade_e_titulos() {
        assert_eq!(perfil(&[]).titulo, "Novato");
        let h = vec![e("C501-D1-1", 1, "Aprovado"), e("C502-D2-2", 2, "Aprovado"), e("C503-D3-3", 3, "Reprovado")];
        let p = perfil(&h);
        assert_eq!(p.nivel, 3);
        assert_eq!(p.titulo, "Aprendiz");
        assert_eq!(p.proximo, Some(("Perito em formação", 7)));
        // o mesmo caso não conta duas vezes; reprovado não conta
        assert_eq!(ganho(&h, "c501-d1-1", 1, true), 0);
        assert_eq!(ganho(&h, "C503-D3-3", 3, true), 3);
        assert_eq!(ganho(&h, "C504-D4-4", 4, false), 0);
        assert_eq!(titulo(50), "Expert");
        assert_eq!(titulo(49), "Especialista");
    }
}
