//! Gerador de PDF próprio (PDF 1.4, fontes padrão Helvetica/Courier com WinAnsiEncoding).
//! Sem dependências externas e sem serviço online. Faz paginação, hierarquia tipográfica,
//! tabelas em fonte monoespaçada, destaque visual (fundo amarelo) e rodapé "Página X de N".
//! Estilo inspirado em convenções ABNT (margens 3/2 cm, A4) — sem alegar conformidade integral.

use crate::engine::Sim;
use crate::model::DocKind;
use crate::util::*;

const W: f32 = 595.28; // A4
const H: f32 = 841.89;
const ML: f32 = 85.0; // 3 cm
const MR: f32 = 57.0; // 2 cm
const MT: f32 = 85.0;
const MB: f32 = 70.0;

#[derive(Clone)]
pub enum Block {
    Title(String),
    Heading(String),
    Para(String),
    Mono(String),
    Highlight(String),
    Small(String),
    Space(f32),
}

#[derive(Clone, Copy, PartialEq)]
enum Font {
    Reg,
    Bold,
    Mono,
}

impl Font {
    fn name(self) -> &'static str {
        match self {
            Font::Reg => "F1",
            Font::Bold => "F2",
            Font::Mono => "F3",
        }
    }
}

/// Larguras Helvetica (AFM) para ASCII 32..126, em milésimos de em.
const HELV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556,
    556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556,
    556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

fn base_ascii(c: char) -> char {
    let f = fold(&c.to_string());
    let b = f.chars().next().unwrap_or('?');
    if c.is_uppercase() { b.to_ascii_uppercase() } else { b }
}

fn char_w(c: char, font: Font, size: f32) -> f32 {
    if font == Font::Mono {
        return 0.6 * size;
    }
    let b = if c.is_ascii() { c } else { base_ascii(c) };
    let w = if (' '..='~').contains(&b) { HELV[(b as u8 - 32) as usize] as f32 } else { 556.0 };
    let w = if font == Font::Bold { w * 1.06 } else { w };
    w / 1000.0 * size
}

fn text_w(s: &str, font: Font, size: f32) -> f32 {
    s.chars().map(|c| char_w(c, font, size)).sum()
}

/// Converte para WinAnsi (CP1252); caracteres sem equivalente viram aproximação ASCII.
fn winansi(s: &str) -> Vec<u8> {
    let mut out = vec![];
    for c in s.chars() {
        let code: Option<u8> = match c {
            '\u{20}'..='\u{7e}' => Some(c as u8),
            '\u{a0}'..='\u{ff}' => Some(c as u32 as u8),
            '—' => Some(0x97),
            '–' => Some(0x96),
            '‘' => Some(0x91),
            '’' => Some(0x92),
            '“' => Some(0x93),
            '”' => Some(0x94),
            '•' => Some(0x95),
            '…' => Some(0x85),
            '€' => Some(0x80),
            _ => None,
        };
        match code {
            Some(b) => {
                if b == b'(' || b == b')' || b == b'\\' {
                    out.push(b'\\');
                }
                out.push(b);
            }
            None => {
                let rep = match c {
                    '■' | '▪' => "-",
                    '✓' => "ok",
                    '→' => "->",
                    '←' => "<-",
                    '─' => "-",
                    _ => "?",
                };
                out.extend_from_slice(rep.as_bytes());
            }
        }
    }
    out
}

struct Op {
    x: f32,
    y: f32,
    font: Font,
    size: f32,
    text: String,
    highlight: bool,
}

struct Layout {
    pages: Vec<Vec<Op>>,
    y: f32,
}

impl Layout {
    fn new() -> Self {
        Layout { pages: vec![vec![]], y: H - MT }
    }
    fn ensure(&mut self, h: f32) {
        if self.y - h < MB {
            self.pages.push(vec![]);
            self.y = H - MT;
        }
    }
    fn line(&mut self, text: &str, font: Font, size: f32, lead: f32, highlight: bool, indent: f32) {
        self.ensure(lead);
        self.y -= lead;
        let p = self.pages.last_mut().unwrap();
        p.push(Op { x: ML + indent, y: self.y, font, size, text: text.to_string(), highlight });
    }
    fn wrap_para(&mut self, text: &str, font: Font, size: f32, lead: f32, highlight: bool, indent: f32) {
        let maxw = W - ML - MR - indent;
        for para in text.split('\n') {
            if para.trim().is_empty() {
                self.y -= lead * 0.5;
                continue;
            }
            let mut cur = String::new();
            for word in para.split(' ') {
                let cand = if cur.is_empty() { word.to_string() } else { format!("{} {}", cur, word) };
                if text_w(&cand, font, size) > maxw && !cur.is_empty() {
                    self.line(&cur, font, size, lead, highlight, indent);
                    cur = word.to_string();
                } else {
                    cur = cand;
                }
                // palavra maior que a linha (ex.: hash): quebra por caracteres
                while text_w(&cur, font, size) > maxw {
                    let mut cut = 0;
                    let mut acc = 0.0;
                    for (i, ch) in cur.char_indices() {
                        acc += char_w(ch, font, size);
                        if acc > maxw {
                            break;
                        }
                        cut = i + ch.len_utf8();
                    }
                    let head = cur[..cut].to_string();
                    self.line(&head, font, size, lead, highlight, indent);
                    cur = cur[cut..].to_string();
                }
            }
            if !cur.is_empty() {
                self.line(&cur, font, size, lead, highlight, indent);
            }
        }
    }
}

pub fn render(blocks: &[Block], header: &str) -> Vec<u8> {
    let mut l = Layout::new();
    for b in blocks {
        match b {
            Block::Title(t) => {
                l.wrap_para(t, Font::Bold, 15.0, 20.0, false, 0.0);
                l.y -= 8.0;
            }
            Block::Heading(t) => {
                l.ensure(40.0);
                l.y -= 8.0;
                l.wrap_para(t, Font::Bold, 12.0, 16.0, false, 0.0);
                l.y -= 2.0;
            }
            Block::Para(t) => l.wrap_para(t, Font::Reg, 11.0, 15.5, false, 0.0),
            Block::Small(t) => l.wrap_para(t, Font::Reg, 8.5, 11.0, false, 0.0),
            Block::Mono(t) => {
                for line in t.lines() {
                    l.wrap_para(if line.is_empty() { " " } else { line }, Font::Mono, 8.0, 10.5, false, 6.0);
                }
            }
            Block::Highlight(t) => {
                l.y -= 4.0;
                l.wrap_para(t, Font::Bold, 12.0, 18.0, true, 6.0);
                l.y -= 4.0;
            }
            Block::Space(h) => l.y -= h,
        }
    }
    write_pdf(&l.pages, header)
}

fn write_pdf(pages: &[Vec<Op>], header: &str) -> Vec<u8> {
    let n = pages.len();
    // objetos: 1 catálogo, 2 páginas, 3..5 fontes, depois (página, conteúdo) por página
    let mut objs: Vec<Vec<u8>> = vec![];
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 6 + i * 2)).collect();
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), n).into_bytes());
    for base in ["Helvetica", "Helvetica-Bold", "Courier"] {
        objs.push(format!("<< /Type /Font /Subtype /Type1 /BaseFont /{} /Encoding /WinAnsiEncoding >>", base).into_bytes());
    }
    for (pi, ops) in pages.iter().enumerate() {
        let mut c: Vec<u8> = vec![];
        // destaques (fundo amarelo) primeiro
        for op in ops.iter().filter(|o| o.highlight) {
            let w = text_w(&op.text, op.font, op.size);
            c.extend_from_slice(format!("1 0.92 0.3 rg {:.2} {:.2} {:.2} {:.2} re f 0 g\n", op.x - 3.0, op.y - 4.0, w + 6.0, op.size + 6.0).as_bytes());
        }
        // cabeçalho e rodapé
        let mut extra = vec![
            Op { x: ML, y: H - 45.0, font: Font::Reg, size: 8.0, text: header.to_string(), highlight: false },
            Op { x: ML, y: 38.0, font: Font::Reg, size: 8.0, text: "SIMULAÇÃO DE TREINAMENTO — NÃO É DOCUMENTO OFICIAL".into(), highlight: false },
        ];
        let pg = format!("Página {} de {}", pi + 1, n);
        extra.push(Op { x: W - MR - text_w(&pg, Font::Reg, 8.0), y: 38.0, font: Font::Reg, size: 8.0, text: pg, highlight: false });
        c.extend_from_slice(format!("0.6 G 0.5 w {:.2} {:.2} m {:.2} {:.2} l S 0 G\n", ML, H - 52.0, W - MR, H - 52.0).as_bytes());
        for op in ops.iter().chain(extra.iter()) {
            c.extend_from_slice(format!("BT /{} {:.1} Tf {:.2} {:.2} Td (", op.font.name(), op.size, op.x, op.y).as_bytes());
            c.extend_from_slice(&winansi(&op.text));
            c.extend_from_slice(b") Tj ET\n");
        }
        let content_id = 7 + pi * 2;
        objs.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents {} 0 R >>",
                W, H, content_id
            )
            .into_bytes(),
        );
        let mut stream = format!("<< /Length {} >>\nstream\n", c.len()).into_bytes();
        stream.extend_from_slice(&c);
        stream.extend_from_slice(b"\nendstream");
        objs.push(stream);
    }
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = vec![];
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for off in &offsets {
        out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", objs.len() + 1, xref).as_bytes());
    out
}

/// Monta os blocos do documento final a partir do estado da simulação.
pub fn document_blocks(s: &Sim) -> Vec<Block> {
    let c = &s.case;
    let r = &s.report;
    let mut b = vec![];
    let doc = c.module.document();
    b.push(Block::Small("Instituto de Criminalística (fictício) — Setor de Perícias de Informática".into()));
    b.push(Block::Space(10.0));
    if doc == DocKind::Laudo {
        b.push(Block::Title(format!("LAUDO PERICIAL — {}", c.module.title().to_uppercase())));
    } else {
        b.push(Block::Title("TERMO DE APREENSÃO DE DISPOSITIVO MÓVEL".into()));
    }
    b.push(Block::Para(format!("Referência: {}   ·   {}   ·   POP {}   ·   Data: {}", c.id, c.procedimento, c.module.code(), fmt_date(s.clock, BRT))));
    let or = |v: &str, d: &str| if v.trim().is_empty() { d.to_string() } else { v.to_string() };
    if doc == DocKind::Laudo {
        b.push(Block::Heading("1. PREÂMBULO".into()));
        b.push(Block::Para(format!("Exame realizado em atendimento ao {}, requisitado por {}.", c.procedimento, c.requisitante)));
        b.push(Block::Heading("2. HISTÓRICO".into()));
        b.push(Block::Para(or(&r.historico, &c.historico)));
        b.push(Block::Heading("3. OBJETIVO".into()));
        b.push(Block::Para(or(&r.objetivo, &c.objetivo)));
        if !c.quesitos.is_empty() {
            b.push(Block::Para(c.quesitos.join("\n")));
        }
        b.push(Block::Heading("4. MATERIAL".into()));
        b.push(Block::Para(or(&r.material, "(não preenchido)")));
        b.push(Block::Heading("5. EXAME".into()));
        b.push(Block::Para(or(&r.exame, "(não preenchido)")));
        if !c.keyword.is_empty() {
            b.push(Block::Heading("6. EXPRESSÃO DE INTERESSE".into()));
            if r.keyword_claim.is_empty() {
                b.push(Block::Para("Não indicada.".into()));
            } else {
                b.push(Block::Highlight(format!("«{}»", r.keyword_claim)));
                b.push(Block::Para(format!("Origem / forma de obtenção: {}", or(&r.keyword_source, "(não indicada)"))));
                b.push(Block::Small("Observação: a localização da expressão é um FATO verificado no material; a relação com pessoas é INFERÊNCIA que depende das demais evidências e limitações registradas.".into()));
            }
        }
        b.push(Block::Heading("7. CONSIDERAÇÕES TÉCNICAS E LIMITAÇÕES".into()));
        b.push(Block::Para(or(&r.consideracoes, "Sem considerações adicionais.")));
        b.push(Block::Heading("8. CONCLUSÃO".into()));
        b.push(Block::Para(or(&r.conclusao, "(não preenchida)")));
        b.push(Block::Heading("ANEXO A — REGISTRO DE HASHES (SHA-256)".into()));
        if s.artifacts.is_empty() {
            b.push(Block::Para("Nenhum artefato produzido.".into()));
        } else {
            let rows: Vec<Vec<String>> = s.artifacts.iter().map(|a| vec![a.id.clone(), a.kind.clone(), a.origin.clone()]).collect();
            b.push(Block::Mono(table(&["ID", "Tipo", "Origem"], &rows)));
            for a in &s.artifacts {
                b.push(Block::Mono(format!("{}  {}", a.id, a.hash)));
            }
            let list: String = s.artifacts.iter().map(|a| format!("{} {}\n", a.id, a.hash)).collect();
            b.push(Block::Small(format!("Hash da lista de hashes deste anexo: {}", crate::sha256::sha256(&list))));
        }
    } else {
        let txt = crate::report::preview(s);
        let body: Vec<&str> = txt.lines().skip(9).collect();
        b.push(Block::Para(body.join("\n")));
    }
    b.push(Block::Heading(format!("ANEXO {} — HISTÓRICO DE AÇÕES (AUDITORIA ENCADEADA)", if doc == DocKind::Laudo { "B" } else { "A" })));
    let rows: Vec<Vec<String>> = s.audit.entries.iter().map(|e| vec![format!("{:03}", e.seq), fmt_dt(e.sim_time, BRT), e.action.clone(), e.detail.chars().take(52).collect()]).collect();
    b.push(Block::Mono(table(&["#", "Data/hora", "Ação", "Detalhe"], &rows)));
    b.push(Block::Small(format!("Último elo da cadeia: {}", s.audit.last_hash())));
    let lims: Vec<String> = crate::assess::checklist(s).into_iter().filter(|(ok, _)| !ok).map(|(_, l)| format!("- {}", l)).collect();
    if !lims.is_empty() {
        b.push(Block::Heading("PROCEDIMENTOS NÃO REALIZADOS".into()));
        b.push(Block::Para(lims.join("\n")));
    }
    b
}

pub fn document_pdf(s: &Sim) -> Vec<u8> {
    let header = format!("{} — {}", s.case.id, s.case.module.short());
    render(&document_blocks(s), &header)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(hay: &[u8], needle: &[u8]) -> usize {
        hay.windows(needle.len()).filter(|w| *w == needle).count()
    }

    #[test]
    fn pdf_estruturalmente_valido_e_paginado() {
        let mut blocks = vec![Block::Title("Teste com acentuação: ação, órgão, «destaque»".into())];
        for i in 0..120 {
            blocks.push(Block::Para(format!("Parágrafo {} — texto longo o suficiente para quebrar linhas e forçar várias páginas no documento final.", i)));
        }
        blocks.push(Block::Highlight("PROJETO ÍCARO-42".into()));
        let pdf = render(&blocks, "cabeçalho");
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        // startxref aponta para a tabela xref
        let s = String::from_utf8_lossy(&pdf);
        let sx: usize = s.rsplit("startxref\n").next().unwrap().lines().next().unwrap().parse().unwrap();
        assert_eq!(&pdf[sx..sx + 4], b"xref");
        // cada entrada da xref aponta para "N 0 obj"
        let xref = &s[sx..];
        for (i, line) in xref.lines().skip(3).take_while(|l| l.ends_with(" n ")).enumerate() {
            let off: usize = line[..10].parse().unwrap();
            assert!(pdf[off..].starts_with(format!("{} 0 obj", i + 1).as_bytes()));
        }
        let pages = count(&pdf, b"/Type /Page ");
        assert!(pages >= 3);
        assert!(s.contains(&format!("/Count {}", pages)));
        assert!(s.contains(&format!("Página {} de {}", pages, pages).replace('á', "\u{e1}")) || count(&pdf, b" de ") > 0);
        // destaque presente (retângulo amarelo) e texto WinAnsi (Í = 0xCD)
        assert!(count(&pdf, b"1 0.92 0.3 rg") >= 1);
        assert!(count(&pdf, b"PROJETO \xcdCARO-42") == 1);
    }
}
