//! Interpretador de linha de comando no estilo bash (subconjunto):
//! aspas simples/duplas, barra invertida, $VAR, $?, ~, curingas * e ?, pipes `|`,
//! redirecionamento `>`, `>>`, `2>`, `2>&1`, `<` e encadeamento `;`, `&&`, `||`.

use super::{cmds, norm, Os, Shell};
use crate::engine::Sim;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Word(String, bool), // (texto, teve aspas → não expandir curinga)
    Pipe,
    And,
    Or,
    Semi,
    Out,
    Append,
    ErrOut,
    ErrToOut,
    In,
}

fn lex(line: &str, sh: &Shell) -> Result<Vec<Tok>, String> {
    let mut toks = vec![];
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut cur = String::new();
    let mut in_word = false;
    let mut quoted = false;
    let flush = |toks: &mut Vec<Tok>, cur: &mut String, in_word: &mut bool, quoted: &mut bool| {
        if *in_word {
            toks.push(Tok::Word(std::mem::take(cur), *quoted));
            *in_word = false;
            *quoted = false;
        }
    };
    let var = |chars: &[char], i: &mut usize, out: &mut String| {
        // chamado com chars[*i] == '$'
        *i += 1;
        if *i < chars.len() && chars[*i] == '?' {
            out.push_str(&sh.last_code.to_string());
            *i += 1;
            return;
        }
        let braced = *i < chars.len() && chars[*i] == '{';
        if braced {
            *i += 1;
        }
        let start = *i;
        while *i < chars.len() && (chars[*i].is_alphanumeric() || chars[*i] == '_') {
            *i += 1;
        }
        let name: String = chars[start..*i].iter().collect();
        if braced && *i < chars.len() && chars[*i] == '}' {
            *i += 1;
        }
        if name.is_empty() {
            out.push('$');
        } else {
            out.push_str(sh.env.get(&name).map(|s| s.as_str()).unwrap_or(""));
        }
    };
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => {
                flush(&mut toks, &mut cur, &mut in_word, &mut quoted);
                i += 1;
            }
            '#' if !in_word => break,
            '\'' => {
                in_word = true;
                quoted = true;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    cur.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return Err("erro de sintaxe: aspas simples não fechadas".into());
                }
                i += 1;
            }
            '"' => {
                in_word = true;
                quoted = true;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() && matches!(chars[i + 1], '"' | '\\' | '$') {
                        cur.push(chars[i + 1]);
                        i += 2;
                    } else if chars[i] == '$' {
                        var(&chars, &mut i, &mut cur);
                    } else {
                        cur.push(chars[i]);
                        i += 1;
                    }
                }
                if i >= chars.len() {
                    return Err("erro de sintaxe: aspas duplas não fechadas".into());
                }
                i += 1;
            }
            '\\' => {
                in_word = true;
                if i + 1 < chars.len() {
                    cur.push(chars[i + 1]);
                }
                i += 2;
            }
            '$' => {
                in_word = true;
                var(&chars, &mut i, &mut cur);
            }
            '~' if !in_word && (i + 1 >= chars.len() || chars[i + 1] == '/' || chars[i + 1] == ' ') => {
                in_word = true;
                cur.push_str(sh.env.get("HOME").map(|s| s.as_str()).unwrap_or("/home/perito"));
                i += 1;
            }
            '|' => {
                flush(&mut toks, &mut cur, &mut in_word, &mut quoted);
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    toks.push(Tok::Or);
                    i += 2;
                } else {
                    toks.push(Tok::Pipe);
                    i += 1;
                }
            }
            '&' => {
                flush(&mut toks, &mut cur, &mut in_word, &mut quoted);
                if i + 1 < chars.len() && chars[i + 1] == '&' {
                    toks.push(Tok::And);
                    i += 2;
                } else {
                    return Err("execução em segundo plano (&) não é suportada nesta estação".into());
                }
            }
            ';' => {
                flush(&mut toks, &mut cur, &mut in_word, &mut quoted);
                toks.push(Tok::Semi);
                i += 1;
            }
            '>' => {
                let err = in_word && cur == "2" && !quoted;
                if err {
                    cur.clear();
                    in_word = false;
                } else {
                    flush(&mut toks, &mut cur, &mut in_word, &mut quoted);
                }
                if err && i + 2 < chars.len() + 1 && chars.get(i + 1) == Some(&'&') && chars.get(i + 2) == Some(&'1') {
                    toks.push(Tok::ErrToOut);
                    i += 3;
                } else if err {
                    toks.push(Tok::ErrOut);
                    i += 1;
                } else if chars.get(i + 1) == Some(&'>') {
                    toks.push(Tok::Append);
                    i += 2;
                } else {
                    toks.push(Tok::Out);
                    i += 1;
                }
            }
            '<' => {
                flush(&mut toks, &mut cur, &mut in_word, &mut quoted);
                toks.push(Tok::In);
                i += 1;
            }
            _ => {
                in_word = true;
                cur.push(c);
                i += 1;
            }
        }
    }
    flush(&mut toks, &mut cur, &mut in_word, &mut quoted);
    Ok(toks)
}

#[derive(Debug, Default, Clone)]
struct Simple {
    argv: Vec<(String, bool)>,
    out: Option<(String, bool)>,
    err: Option<String>,
    err_to_out: bool,
    input: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Conn {
    Seq,
    And,
    Or,
}

fn parse(toks: Vec<Tok>) -> Result<Vec<(Vec<Simple>, Conn)>, String> {
    let mut list = vec![];
    let mut pipeline: Vec<Simple> = vec![];
    let mut cur = Simple::default();
    let mut it = toks.into_iter().peekable();
    let target = |it: &mut std::iter::Peekable<std::vec::IntoIter<Tok>>| -> Result<String, String> {
        match it.next() {
            Some(Tok::Word(w, _)) => Ok(w),
            _ => Err("erro de sintaxe próximo ao token inesperado `newline'".into()),
        }
    };
    let end = |pipeline: &mut Vec<Simple>, cur: &mut Simple, list: &mut Vec<(Vec<Simple>, Conn)>, c: Conn| -> Result<(), String> {
        if cur.argv.is_empty() {
            if pipeline.is_empty() && c == Conn::Seq {
                return Ok(());
            }
            return Err("erro de sintaxe próximo ao token inesperado".into());
        }
        pipeline.push(std::mem::take(cur));
        list.push((std::mem::take(pipeline), c));
        Ok(())
    };
    while let Some(t) = it.next() {
        match t {
            Tok::Word(w, q) => cur.argv.push((w, q)),
            Tok::Pipe => {
                if cur.argv.is_empty() {
                    return Err("erro de sintaxe próximo ao token inesperado `|'".into());
                }
                pipeline.push(std::mem::take(&mut cur));
            }
            Tok::Out => cur.out = Some((target(&mut it)?, false)),
            Tok::Append => cur.out = Some((target(&mut it)?, true)),
            Tok::ErrOut => cur.err = Some(target(&mut it)?),
            Tok::ErrToOut => cur.err_to_out = true,
            Tok::In => cur.input = Some(target(&mut it)?),
            Tok::Semi => end(&mut pipeline, &mut cur, &mut list, Conn::Seq)?,
            Tok::And => end(&mut pipeline, &mut cur, &mut list, Conn::And)?,
            Tok::Or => end(&mut pipeline, &mut cur, &mut list, Conn::Or)?,
        }
    }
    if !cur.argv.is_empty() || !pipeline.is_empty() {
        end(&mut pipeline, &mut cur, &mut list, Conn::Seq)?;
    }
    Ok(list)
}

/// Correspondência de curinga simples (* e ?).
pub fn glob_match(pat: &str, s: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = s.chars().collect();
    fn m(p: &[char], t: &[char]) -> bool {
        match (p.first(), t.first()) {
            (None, None) => true,
            (Some('*'), _) => m(&p[1..], t) || (!t.is_empty() && m(p, &t[1..])),
            (Some('?'), Some(_)) => m(&p[1..], &t[1..]),
            (Some(a), Some(b)) if a == b => m(&p[1..], &t[1..]),
            _ => false,
        }
    }
    m(&p, &t)
}

fn expand_glob(os: &Os, sim: &Sim, sh: &Shell, w: &str) -> Vec<String> {
    if !w.contains('*') && !w.contains('?') {
        return vec![w.to_string()];
    }
    // expande segmento a segmento (como o bash): /a/*/b/*.txt
    let absolute = w.starts_with('/');
    let segs: Vec<&str> = w.split('/').filter(|x| !x.is_empty()).collect();
    // cada candidato: (caminho exibido, caminho absoluto)
    let mut cands: Vec<(String, String)> = vec![(if absolute { "/".into() } else { String::new() }, if absolute { "/".into() } else { sh.cwd.clone() })];
    for (i, seg) in segs.iter().enumerate() {
        let last = i + 1 == segs.len();
        let mut next = vec![];
        for (shown, abs_dir) in &cands {
            let join = |base: &str, n: &str| -> String { if base.is_empty() { n.to_string() } else if base.ends_with('/') { format!("{}{}", base, n) } else { format!("{}/{}", base, n) } };
            if seg.contains('*') || seg.contains('?') {
                let Ok(entries) = os.list(sim, abs_dir) else { continue };
                let mut names: Vec<(String, bool)> = entries
                    .into_iter()
                    .filter(|(n, _)| (seg.starts_with('.') || !n.starts_with('.')) && glob_match(seg, n))
                    .map(|(n, k)| (n, k.is_dir()))
                    .collect();
                names.sort();
                for (n, is_dir) in names {
                    if !last && !is_dir {
                        continue;
                    }
                    next.push((join(shown, &n), join(abs_dir, &n)));
                }
            } else {
                let a = join(abs_dir, seg);
                if last || os.kind(sim, &a).map(|k| k.is_dir()).unwrap_or(false) {
                    next.push((join(shown, seg), a));
                }
            }
        }
        cands = next;
        if cands.is_empty() {
            return vec![w.to_string()];
        }
    }
    let out: Vec<String> = cands.into_iter().filter(|(_, a)| os.kind(sim, a).is_some()).map(|(s, _)| s).collect();
    if out.is_empty() {
        vec![w.to_string()]
    } else {
        out
    }
}

pub struct Out {
    pub out: String,
    pub err: String,
    pub code: i32,
}

impl Out {
    pub fn ok(s: impl Into<String>) -> Out {
        Out { out: s.into(), err: String::new(), code: 0 }
    }
    pub fn fail(cmd: &str, msg: impl Into<String>) -> Out {
        Out { out: String::new(), err: format!("{}: {}\n", cmd, msg.into()), code: 1 }
    }
    pub fn code(mut self, c: i32) -> Out {
        self.code = c;
        self
    }
}

/// Executa uma linha de comando. Retorna o texto combinado (stdout + stderr) para o terminal.
pub fn run_line(os: &mut Os, sim: &mut Sim, sh: &mut Shell, line: &str) -> String {
    let line = line.trim();
    if line.is_empty() {
        return String::new();
    }
    sh.history.push(line.to_string());
    let toks = match lex(line, sh) {
        Ok(t) => t,
        Err(e) => {
            sh.last_code = 2;
            return format!("bash: {}\n", e);
        }
    };
    let list = match parse(toks) {
        Ok(l) => l,
        Err(e) => {
            sh.last_code = 2;
            return format!("bash: {}\n", e);
        }
    };
    let mut screen = String::new();
    let mut prev_conn = Conn::Seq;
    let mut skip = false;
    for (pipeline, conn) in list {
        let run = match prev_conn {
            Conn::Seq => true,
            Conn::And => !skip && sh.last_code == 0,
            Conn::Or => !skip && sh.last_code != 0,
        };
        // && / ||: se não executar, propaga o pulo dentro da cadeia
        if !run {
            skip = matches!(prev_conn, Conn::And | Conn::Or) && conn != Conn::Seq;
            prev_conn = conn;
            if conn == Conn::Seq {
                skip = false;
            }
            continue;
        }
        skip = false;
        let mut stdin = String::new();
        let n = pipeline.len();
        for (k, simple) in pipeline.into_iter().enumerate() {
            let mut argv: Vec<String> = vec![];
            for (w, q) in &simple.argv {
                if *q {
                    argv.push(w.clone());
                } else {
                    argv.extend(expand_glob(os, sim, sh, w));
                }
            }
            if let Some(inp) = &simple.input {
                let p = abs(sh, inp);
                match os.read(sim, &p) {
                    Ok(d) => stdin = d,
                    Err(e) => {
                        screen.push_str(&format!("bash: {}: {}\n", inp, e));
                        sh.last_code = 1;
                        break;
                    }
                }
            }
            let r = cmds::dispatch(os, sim, sh, &argv, &stdin);
            let mut out = r.out;
            let mut err = r.err;
            if simple.err_to_out {
                out.push_str(&err);
                err.clear();
            }
            if let Some(ep) = &simple.err {
                let p = abs(sh, ep);
                if p != "/dev/null" {
                    let _ = os.write(sim, &p, &err, false, "stderr");
                }
                err.clear();
            }
            screen.push_str(&err);
            if let Some((op, app)) = &simple.out {
                let p = abs(sh, op);
                if p != "/dev/null" {
                    let origin = format!("saída de: {}", argv.join(" "));
                    if let Err(e) = os.write(sim, &p, &out, *app, &origin) {
                        screen.push_str(&format!("bash: {}: {}\n", op, e));
                    }
                }
                out.clear();
            }
            sh.last_code = r.code;
            if k + 1 < n {
                stdin = out;
            } else {
                screen.push_str(&out);
            }
        }
        prev_conn = conn;
    }
    if !screen.is_empty() && !screen.ends_with('\n') {
        screen.push('\n');
    }
    screen
}

pub fn abs(sh: &Shell, p: &str) -> String {
    if p.starts_with('/') {
        norm(p)
    } else {
        norm(&format!("{}/{}", sh.cwd, p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lexer_aspas_variaveis_operadores() {
        let mut sh = Shell::new("/home/perito");
        sh.env.insert("X".into(), "valor".into());
        let t = lex(r#"echo "a b" 'c $X' $X ~/d | grep -i x > o.txt 2>&1 && ls || pwd; cd"#, &sh).unwrap();
        assert_eq!(t[1], Tok::Word("a b".into(), true));
        assert_eq!(t[2], Tok::Word("c $X".into(), true));
        assert_eq!(t[3], Tok::Word("valor".into(), false));
        assert_eq!(t[4], Tok::Word("/home/perito/d".into(), false));
        assert!(t.contains(&Tok::Pipe) && t.contains(&Tok::Out) && t.contains(&Tok::ErrToOut) && t.contains(&Tok::And) && t.contains(&Tok::Or) && t.contains(&Tok::Semi));
        assert!(lex("echo 'aberta", &sh).is_err());
    }
    #[test]
    fn curingas() {
        assert!(glob_match("*.docx", "a.docx"));
        assert!(!glob_match("*.docx", "a.doc"));
        assert!(glob_match("IMG_????.jpg", "IMG_2016.jpg"));
    }
}
