//! Datas, formatação e pequenas utilidades. O tempo do simulador é medido em
//! segundos UTC (i64). Exibição padrão: horário de Brasília (UTC-03:00).

pub const BRT: i32 = -180; // deslocamento em minutos

/// Dias desde 1970-01-01 → (ano, mês, dia). Algoritmo de H. Hinnant.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Instante UTC a partir de data/hora local com deslocamento (minutos).
pub fn ts(y: i64, mo: u32, d: u32, h: u32, mi: u32, offset_min: i32) -> i64 {
    days_from_civil(y, mo, d) * 86400 + (h as i64) * 3600 + (mi as i64) * 60 - (offset_min as i64) * 60
}

pub struct Parts {
    pub y: i64,
    pub mo: u32,
    pub d: u32,
    pub h: u32,
    pub mi: u32,
    pub s: u32,
    pub wd: u32,
}

pub fn parts(t: i64, offset_min: i32) -> Parts {
    let l = t + offset_min as i64 * 60;
    let days = l.div_euclid(86400);
    let sec = l.rem_euclid(86400);
    let (y, mo, d) = civil_from_days(days);
    Parts { y, mo, d, h: (sec / 3600) as u32, mi: ((sec % 3600) / 60) as u32, s: (sec % 60) as u32, wd: ((days + 4).rem_euclid(7)) as u32 }
}

pub fn fmt_offset(off: i32) -> String {
    let sign = if off < 0 { '-' } else { '+' };
    let a = off.abs();
    format!("UTC{}{:02}:{:02}", sign, a / 60, a % 60)
}

pub fn fmt_dt(t: i64, off: i32) -> String {
    let p = parts(t, off);
    format!("{:02}/{:02}/{} {:02}:{:02}:{:02}", p.d, p.mo, p.y, p.h, p.mi, p.s)
}

pub fn fmt_dtz(t: i64, off: i32) -> String {
    format!("{} ({})", fmt_dt(t, off), fmt_offset(off))
}

pub fn fmt_date(t: i64, off: i32) -> String {
    let p = parts(t, off);
    format!("{:02}/{:02}/{}", p.d, p.mo, p.y)
}

pub fn fmt_hm(t: i64, off: i32) -> String {
    let p = parts(t, off);
    format!("{:02}:{:02}", p.h, p.mi)
}

pub fn fmt_iso(t: i64) -> String {
    let p = parts(t, 0);
    format!("{}-{:02}-{:02}T{:02}:{:02}:{:02}Z", p.y, p.mo, p.d, p.h, p.mi, p.s)
}

/// Formato de cabeçalho de e-mail (RFC 5322).
pub fn fmt_rfc5322(t: i64, off: i32) -> String {
    const D: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let p = parts(t, off);
    let sign = if off < 0 { '-' } else { '+' };
    let a = off.abs();
    format!("{}, {:02} {} {} {:02}:{:02}:{:02} {}{:02}{:02}", D[p.wd as usize], p.d, M[(p.mo - 1) as usize], p.y, p.h, p.mi, p.s, sign, a / 60, a % 60)
}

pub fn fmt_bytes(n: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1000.0 && i < U.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 {
        format!("{} B", n)
    } else if v < 10.0 {
        format!("{:.2} {}", v, U[i])
    } else {
        format!("{:.1} {}", v, U[i])
    }
}

pub fn fmt_duration(secs: i64) -> String {
    let m = secs / 60;
    if m < 60 {
        format!("{} min", m.max(if secs > 0 { 1 } else { 0 }))
    } else if m % 60 == 0 {
        format!("{} h", m / 60)
    } else {
        format!("{} h {} min", m / 60, m % 60)
    }
}

/// Remove acentos e passa para minúsculas — para buscas tolerantes.
pub fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// Tabela de texto alinhada (saída do terminal).
pub fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let cols = header.len();
    let mut w: Vec<usize> = header.iter().map(|h| h.chars().count()).collect();
    for r in rows {
        for c in 0..cols.min(r.len()) {
            w[c] = w[c].max(r[c].chars().count());
        }
    }
    let line = |cells: Vec<String>| -> String {
        let mut s = String::new();
        for (c, x) in cells.iter().enumerate() {
            s.push_str(x);
            if c + 1 < cells.len() {
                s.push_str(&" ".repeat(w[c] - x.chars().count() + 2));
            }
        }
        s.trim_end().to_string()
    };
    let mut out = vec![line(header.iter().map(|s| s.to_string()).collect())];
    out.push(w.iter().map(|n| "-".repeat(*n)).collect::<Vec<_>>().join("  "));
    for r in rows {
        out.push(line(r.clone()));
    }
    out.join("\n")
}

pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![];
    for para in text.split('\n') {
        let mut cur = String::new();
        for word in para.split(' ') {
            if cur.chars().count() + word.chars().count() + 1 > width && !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
        }
        lines.push(cur);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn datas_ida_e_volta() {
        let t = ts(2026, 3, 14, 9, 30, BRT);
        assert_eq!(fmt_dt(t, BRT), "14/03/2026 09:30:00");
        assert_eq!(fmt_dt(t, 0), "14/03/2026 12:30:00");
        assert_eq!(fmt_iso(t), "2026-03-14T12:30:00Z");
        assert_eq!(fmt_rfc5322(t, BRT), "Sat, 14 Mar 2026 09:30:00 -0300");
        for d in [-1000i64, 0, 59, 20000, 30000] {
            let (y, m, dd) = civil_from_days(d);
            assert_eq!(days_from_civil(y, m, dd), d);
        }
    }
    #[test]
    fn fold_remove_acentos() {
        assert_eq!(fold("Operação ÍCARO"), "operacao icaro");
    }
}

/// Representação estável de uma mídia (sem partições) para hashing e serialização.
pub fn stable_media_repr(m: &crate::model::Media) -> String {
    format!("MEDIA|{}|{}|{}|{}|{}|{}", m.id, m.brand, m.model, m.serial, m.capacity, m.kind.label())
}

/// Distância de edição (Levenshtein) entre duas palavras curtas.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}
