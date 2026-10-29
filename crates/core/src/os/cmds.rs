//! Comandos de uso geral do Perícia Linux (subconjunto de coreutils/util-linux),
//! com mensagens no padrão do GNU em pt_BR. As operações periciais e a rede emulada são
//! acessadas pelo comando `pericia`, que encaminha ao motor do simulador.

use super::shell::{abs, glob_match, Out};
use super::{basename, dirname, man, norm, Kind, Node, Os, Shell, DISTRO, DISTRO_VERSION, KERNEL};
use crate::engine::Sim;
use crate::osapi::fmt_ls_time;
use crate::util::*;

const GENERAL: &[&str] = &[
    "ls", "cd", "pwd", "cat", "less", "more", "head", "tail", "grep", "wc", "echo", "touch", "mkdir", "rmdir", "rm", "cp", "mv", "file", "stat", "find", "sort", "uniq", "tree",
    "whoami", "id", "hostname", "uname", "date", "clear", "history", "man", "help", "which", "env", "export", "lsblk", "mount", "umount", "df", "free", "ps", "uptime",
    "sha256sum", "sudo", "exit", "logout", "dmesg", "ajuda-pericia", "pericia",
];

pub fn names() -> Vec<&'static str> {
    let mut v: Vec<&str> = GENERAL.to_vec();
    v.sort();
    v
}

pub fn exists(c: &str) -> bool {
    GENERAL.contains(&c)
}

/// Separa opções curtas/longas dos operandos.
pub fn opts(args: &[String]) -> (String, Vec<String>, Vec<String>) {
    let mut short = String::new();
    let mut long = vec![];
    let mut pos = vec![];
    let mut only_pos = false;
    for a in args {
        if only_pos {
            pos.push(a.clone());
        } else if a == "--" {
            only_pos = true;
        } else if let Some(l) = a.strip_prefix("--") {
            long.push(l.to_string());
        } else if a.starts_with('-') && a.len() > 1 && !a[1..].chars().all(|c| c.is_ascii_digit()) {
            short.push_str(&a[1..]);
        } else {
            pos.push(a.clone());
        }
    }
    (short, long, pos)
}

pub fn dispatch(os: &mut Os, sim: &mut Sim, sh: &mut Shell, argv: &[String], stdin: &str) -> Out {
    let Some(cmd) = argv.first() else { return Out::ok("") };
    let mut cmd = cmd.as_str();
    let mut args: Vec<String> = argv[1..].to_vec();
    if cmd == "sudo" {
        if args.is_empty() {
            return Out::fail("sudo", "uso: sudo comando");
        }
        cmd = Box::leak(args.remove(0).into_boxed_str());
    }
    if let Some(stripped) = cmd.strip_prefix("/usr/bin/").or_else(|| cmd.strip_prefix("/bin/")) {
        cmd = Box::leak(stripped.to_string().into_boxed_str());
    }
    // aliases do ~/.bashrc
    if cmd == "ll" {
        args.insert(0, "-l".into());
        cmd = "ls";
    } else if cmd == "la" {
        args.insert(0, "-la".into());
        cmd = "ls";
    }
    match cmd {
        "ls" => ls(os, sim, sh, &args),
        "cd" => cd(os, sim, sh, &args),
        "pwd" => Out::ok(format!("{}\n", sh.cwd)),
        "cat" | "less" | "more" => cat(os, sim, sh, cmd, &args, stdin),
        "head" | "tail" => head_tail(os, sim, sh, cmd, &args, stdin),
        "grep" => grep(os, sim, sh, &args, stdin),
        "wc" => wc(os, sim, sh, &args, stdin),
        "echo" => {
            let (s, _, p) = opts(&args);
            let txt = p.join(" ");
            Out::ok(if s.contains('n') { txt } else { format!("{}\n", txt) })
        }
        "touch" => touch(os, sim, sh, &args),
        "mkdir" => mkdir(os, sim, sh, &args),
        "rmdir" | "rm" => rm(os, sim, sh, cmd, &args),
        "cp" | "mv" => cp_mv(os, sim, sh, cmd, &args),
        "file" => file(os, sim, sh, &args),
        "stat" => stat(os, sim, sh, &args),
        "find" => find(os, sim, sh, &args),
        "sort" | "uniq" => sort_uniq(os, sim, sh, cmd, &args, stdin),
        "tree" => tree(os, sim, sh, &args),
        "whoami" => Out::ok("perito\n"),
        "id" => Out::ok("uid=1000(perito) gid=1000(perito) grupos=1000(perito),6(disk),27(sudo)\n"),
        "hostname" => Out::ok(format!("{}\n", os.hostname)),
        "uname" => {
            let (s, _, _) = opts(&args);
            if s.contains('a') {
                Out::ok(format!("Linux {} {} #1 SMP PREEMPT_DYNAMIC Debian 6.1.119-1 x86_64 GNU/Linux\n", os.hostname, KERNEL))
            } else if s.contains('r') {
                Out::ok(format!("{}\n", KERNEL))
            } else {
                Out::ok("Linux\n")
            }
        }
        "date" => {
            let (s, _, _) = opts(&args);
            if s.contains('u') {
                Out::ok(format!("{} UTC\n", fmt_dt(sim.clock, 0)))
            } else {
                Out::ok(format!("{} -03\n", fmt_dt(sim.clock, BRT)))
            }
        }
        "clear" => {
            sh.clear_requested = true;
            Out::ok("")
        }
        "history" => {
            let mut s = String::new();
            for (i, h) in sh.history.iter().enumerate() {
                s.push_str(&format!("{:>5}  {}\n", i + 1, h));
            }
            Out::ok(s)
        }
        "man" => match args.first() {
            None => Out::fail("man", "Qual página de manual você quer?\nPor exemplo, tente 'man man' ou 'man intro'."),
            Some(p) => match man::page(p) {
                Some(t) => Out::ok(t),
                None => Out::fail("man", format!("nenhuma entrada de manual para {}", p)).code(16),
            },
        },
        "help" | "ajuda-pericia" => Out::ok(man::page("intro").unwrap_or_default()),
        "which" => {
            let mut s = String::new();
            let mut code = 0;
            for a in &args {
                if exists(a) {
                    s.push_str(&format!("/usr/bin/{}\n", a));
                } else {
                    code = 1;
                }
            }
            Out::ok(s).code(code)
        }
        "env" => Out::ok(sh.env.iter().map(|(k, v)| format!("{}={}\n", k, v)).collect::<String>()),
        "export" => {
            for a in &args {
                if let Some((k, v)) = a.split_once('=') {
                    sh.env.insert(k.to_string(), v.to_string());
                }
            }
            Out::ok("")
        }
        "lsblk" => lsblk(os, sim),
        "mount" => mount(os, sim, sh, &args),
        "umount" => umount(os, sim, sh, &args),
        "df" => df(os, sim),
        "free" => Out::ok("               total        usada        livre    compart.  buff/cache   disponível\nMem.:           31Gi        2,1Gi         27Gi        96Mi        1,9Gi         28Gi\nSwap:          8,0Gi           0B        8,0Gi\n"),
        "ps" => Out::ok("  PID TTY          TIME CMD\n    1 ?        00:00:01 systemd\n  812 ?        00:00:00 Xorg\n  990 ?        00:00:02 xfce4-session\n 1204 pts/0    00:00:00 bash\n 1310 pts/0    00:00:00 ps\n"),
        "uptime" => {
            let up = (sim.clock - os.boot) / 60;
            Out::ok(format!(" {} até {}:{:02},  1 usuário,  carga média: 0,08, 0,05, 0,01\n", fmt_hm(sim.clock, BRT), up / 60, up % 60))
        }
        "sha256sum" => sha256sum(os, sim, sh, &args, stdin),
        "dmesg" => dmesg(os, sim),
        "exit" | "logout" => {
            sh.exit_requested = true;
            Out::ok("logout\n")
        }
        "pericia" => {
            if args.is_empty() {
                return Out::ok("uso: pericia <comando do simulador>   (ex.: pericia caso | pericia achei <expressão> | pericia laudo | pericia finalizar)\n");
            }
            let line = args.join(" ");
            let r = sim.exec(&line);
            os.sync(sim);
            Out { out: format!("{}\n", r.text), err: String::new(), code: if r.ok { 0 } else { 1 } }
        }
        _ => Out { out: String::new(), err: format!("bash: {}: comando não encontrado\n", cmd), code: 127 },
    }
}

// ------------------------------------------------------------------ ls / cd / cat

fn perms(k: &Kind) -> String {
    match k {
        Kind::Dir => "drwxr-xr-x".into(),
        Kind::MDir { .. } => "dr-xr-xr-x".into(),
        Kind::File { mode, .. } => {
            if *mode & 0o111 != 0 {
                "-rwxr-xr-x".into()
            } else {
                "-rw-r--r--".into()
            }
        }
        Kind::Image { .. } => "-r--r-----".into(),
        Kind::Dev { .. } => "brw-rw----".into(),
        Kind::Proc => "-r--r--r--".into(),
        Kind::MFile { .. } => "-r--r--r--".into(),
    }
}

fn size_mtime(os: &Os, k: &Kind) -> (u64, i64) {
    match k {
        Kind::File { size, mtime, .. } => (*size, *mtime),
        Kind::Image { size, mtime, .. } => (*size, *mtime),
        Kind::MFile { size, mtime, .. } => (*size, *mtime),
        Kind::Dev { size, .. } => (*size, os.boot),
        _ => (4096, os.boot),
    }
}

fn owner(k: &Kind) -> String {
    match k {
        Kind::MFile { entry: Some(e), .. } => e.owner.chars().take(8).collect(),
        Kind::MFile { .. } | Kind::MDir { .. } => "root".into(),
        Kind::Dev { .. } => "root".into(),
        _ => "perito".into(),
    }
}

fn hsize(n: u64) -> String {
    let u = ["", "K", "M", "G", "T"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < 4 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{}", n)
    } else if v < 10.0 {
        format!("{:.1}{}", v, u[i]).replace('.', ",")
    } else {
        format!("{:.0}{}", v, u[i])
    }
}

fn ls(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String]) -> Out {
    let (s, _, mut paths) = opts(args);
    let long = s.contains('l');
    let all = s.contains('a');
    let human = s.contains('h');
    if paths.is_empty() {
        paths.push(".".into());
    }
    let mut out = String::new();
    let mut err = String::new();
    let multi = paths.len() > 1;
    for (pi, p) in paths.iter().enumerate() {
        let a = abs(sh, p);
        let Some(k) = os.kind(sim, &a) else {
            err.push_str(&format!("ls: não foi possível acessar '{}': Arquivo ou diretório inexistente\n", p));
            continue;
        };
        let entries: Vec<(String, Kind)> = if k.is_dir() {
            match os.list(sim, &a) {
                Ok(e) => e,
                Err(e) => {
                    err.push_str(&format!("ls: não foi possível abrir o diretório '{}': {}\n", p, e));
                    continue;
                }
            }
        } else {
            vec![(p.clone(), k)]
        };
        if multi {
            out.push_str(&format!("{}{}:\n", if pi > 0 { "\n" } else { "" }, p));
        }
        let mut entries: Vec<(String, Kind)> = entries.into_iter().filter(|(n, _)| all || !n.starts_with('.')).collect();
        if all && k_is_dir(os, sim, &a) {
            entries.insert(0, ("..".into(), Kind::Dir));
            entries.insert(0, (".".into(), Kind::Dir));
        }
        if long {
            let total: u64 = entries.iter().map(|(_, k)| size_mtime(os, k).0 / 1024).sum();
            out.push_str(&format!("total {}\n", total.max(entries.len() as u64 * 4)));
            for (n, k) in &entries {
                let (sz, mt) = size_mtime(os, k);
                let szs = if human { hsize(sz) } else { sz.to_string() };
                let o = owner(k);
                let name = if n.contains(' ') { format!("'{}'", n) } else { n.clone() };
                out.push_str(&format!("{} {} {:<8} {:<8} {:>12} {} {}\n", perms(k), if k.is_dir() { 2 } else { 1 }, o, if o == "perito" { "perito" } else { "root" }, szs, fmt_ls_time(mt, BRT), name));
            }
        } else {
            let names: Vec<String> = entries.iter().map(|(n, k)| {
                let n = if n.contains(' ') { format!("'{}'", n) } else { n.clone() };
                if k.is_dir() { format!("{}/", n) } else { n }
            }).collect();
            out.push_str(&columns(&names, 96));
        }
    }
    Out { out, code: if err.is_empty() { 0 } else { 2 }, err }
}

fn k_is_dir(os: &Os, sim: &Sim, a: &str) -> bool {
    os.kind(sim, a).map(|k| k.is_dir()).unwrap_or(false)
}

pub fn columns(names: &[String], width: usize) -> String {
    if names.is_empty() {
        return String::new();
    }
    let w = names.iter().map(|n| n.chars().count()).max().unwrap_or(1) + 2;
    let cols = (width / w).max(1);
    let rows = names.len().div_ceil(cols);
    let mut out = String::new();
    for r in 0..rows {
        let mut line = String::new();
        for c in 0..cols {
            if let Some(n) = names.get(c * rows + r) {
                line.push_str(n);
                line.push_str(&" ".repeat(w - n.chars().count()));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn cd(os: &mut Os, sim: &mut Sim, sh: &mut Shell, args: &[String]) -> Out {
    let target = match args.first().map(|s| s.as_str()) {
        None | Some("~") => sh.env.get("HOME").cloned().unwrap_or_else(|| os.home.clone()),
        Some("-") => sh.env.get("OLDPWD").cloned().unwrap_or_else(|| sh.cwd.clone()),
        Some(p) => abs(sh, p),
    };
    match os.kind(sim, &target) {
        Some(k) if k.is_dir() => {
            sh.env.insert("OLDPWD".into(), sh.cwd.clone());
            sh.cwd = target.clone();
            sh.env.insert("PWD".into(), target);
            Out::ok("")
        }
        Some(_) => Out::fail("bash: cd", format!("{}: Não é um diretório", args[0])),
        None => Out::fail("bash: cd", format!("{}: Arquivo ou diretório inexistente", args.first().cloned().unwrap_or_default())),
    }
}

/// Lê arquivo (local, de evidência montada, /proc…). Encapsula mensagens de erro.
pub fn read_path(os: &mut Os, sim: &mut Sim, sh: &Shell, cmd: &str, p: &str) -> Result<String, String> {
    let a = abs(sh, p);
    match os.kind(sim, &a) {
        Some(k) if k.is_dir() => Err(format!("{}: {}: É um diretório", cmd, p)),
        Some(_) => os.read(sim, &a).map_err(|e| format!("{}: {}: {}", cmd, p, e)),
        None => Err(format!("{}: {}: Arquivo ou diretório inexistente", cmd, p)),
    }
}

fn cat(os: &mut Os, sim: &mut Sim, sh: &Shell, cmd: &str, args: &[String], stdin: &str) -> Out {
    let (s, _, paths) = opts(args);
    if paths.is_empty() {
        return Out::ok(stdin.to_string());
    }
    let mut out = String::new();
    let mut err = String::new();
    for p in &paths {
        match read_path(os, sim, sh, cmd, p) {
            Ok(d) => {
                if s.contains('n') {
                    for (i, l) in d.lines().enumerate() {
                        out.push_str(&format!("{:>6}\t{}\n", i + 1, l));
                    }
                } else {
                    out.push_str(&d);
                    if !d.ends_with('\n') && !d.is_empty() {
                        out.push('\n');
                    }
                }
            }
            Err(e) => err.push_str(&format!("{}\n", e)),
        }
    }
    Out { out, code: if err.is_empty() { 0 } else { 1 }, err }
}

fn input_or_files(os: &mut Os, sim: &mut Sim, sh: &Shell, cmd: &str, paths: &[String], stdin: &str) -> (Vec<(String, String)>, String) {
    if paths.is_empty() {
        return (vec![(String::new(), stdin.to_string())], String::new());
    }
    let mut v = vec![];
    let mut err = String::new();
    for p in paths {
        match read_path(os, sim, sh, cmd, p) {
            Ok(d) => v.push((p.clone(), d)),
            Err(e) => err.push_str(&format!("{}\n", e)),
        }
    }
    (v, err)
}

fn head_tail(os: &mut Os, sim: &mut Sim, sh: &Shell, cmd: &str, args: &[String], stdin: &str) -> Out {
    let mut n = 10usize;
    let mut rest = vec![];
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-n" && i + 1 < args.len() {
            n = args[i + 1].parse().unwrap_or(10);
            i += 2;
            continue;
        }
        if let Some(x) = args[i].strip_prefix('-') {
            if let Ok(v) = x.parse() {
                n = v;
                i += 1;
                continue;
            }
        }
        rest.push(args[i].clone());
        i += 1;
    }
    let (inputs, err) = input_or_files(os, sim, sh, cmd, &rest, stdin);
    let mut out = String::new();
    for (name, d) in &inputs {
        if inputs.len() > 1 {
            out.push_str(&format!("==> {} <==\n", name));
        }
        let lines: Vec<&str> = d.lines().collect();
        let sel: Vec<&str> = if cmd == "head" { lines.iter().take(n).copied().collect() } else { lines.iter().skip(lines.len().saturating_sub(n)).copied().collect() };
        for l in sel {
            out.push_str(l);
            out.push('\n');
        }
    }
    Out { out, code: if err.is_empty() { 0 } else { 1 }, err }
}

fn grep(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String], stdin: &str) -> Out {
    let (s, long, pos) = opts(args);
    let Some(pat) = pos.first().cloned() else { return Out::fail("grep", "uso: grep [OPÇÃO]... PADRÕES [ARQUIVO]...").code(2) };
    let icase = s.contains('i') || long.iter().any(|l| l == "ignore-case");
    let recursive = s.contains('r') || s.contains('R');
    let numbers = s.contains('n');
    let only_names = s.contains('l');
    let invert = s.contains('v');
    let count = s.contains('c');
    let needle = if icase { fold(&pat) } else { pat.clone() };
    let matches = |l: &str| -> bool {
        let hit = if icase { fold(l).contains(&needle) } else { l.contains(&needle) };
        hit != invert
    };
    let mut files: Vec<String> = pos[1..].to_vec();
    if files.is_empty() && recursive {
        files.push(".".into());
    }
    let mut out = String::new();
    let mut err = String::new();
    let mut any = false;
    if files.is_empty() {
        let mut c = 0;
        for (i, l) in stdin.lines().enumerate() {
            if matches(l) {
                any = true;
                c += 1;
                if !count {
                    out.push_str(&if numbers { format!("{}:{}\n", i + 1, l) } else { format!("{}\n", l) });
                }
            }
        }
        if count {
            out.push_str(&format!("{}\n", c));
        }
        return Out { out, err, code: if any { 0 } else { 1 } };
    }
    // expande diretórios recursivamente
    let mut targets: Vec<(String, String)> = vec![]; // (exibição, abs)
    for f in &files {
        let a = abs(sh, f);
        match os.kind(sim, &a) {
            Some(k) if k.is_dir() => {
                if recursive {
                    for (path, k2) in walk(os, sim, &a, 12) {
                        if !k2.is_dir() && !matches!(k2, Kind::MFile { entry: None, .. }) && !matches!(k2, Kind::Image { .. } | Kind::Dev { .. }) {
                            let shown = if f == "." { format!(".{}", &path[a.len()..]) } else { format!("{}{}", f.trim_end_matches('/'), &path[a.len()..]) };
                            targets.push((shown, path));
                        }
                    }
                } else {
                    err.push_str(&format!("grep: {}: É um diretório\n", f));
                }
            }
            Some(_) => targets.push((f.clone(), a)),
            None => err.push_str(&format!("grep: {}: Arquivo ou diretório inexistente\n", f)),
        }
    }
    let show_name = targets.len() > 1 || recursive;
    for (shown, a) in targets {
        let data = match os.read(sim, &a) {
            Ok(d) => d,
            Err(e) => {
                err.push_str(&format!("grep: {}: {}\n", shown, e));
                continue;
            }
        };
        let mut c = 0;
        for (i, l) in data.lines().enumerate() {
            if matches(l) {
                any = true;
                c += 1;
                if only_names || count {
                    continue;
                }
                let mut line = String::new();
                if show_name {
                    line.push_str(&format!("{}:", shown));
                }
                if numbers {
                    line.push_str(&format!("{}:", i + 1));
                }
                line.push_str(l);
                out.push_str(&line);
                out.push('\n');
            }
        }
        if only_names && c > 0 {
            out.push_str(&format!("{}\n", shown));
        }
        if count {
            out.push_str(&if show_name { format!("{}:{}\n", shown, c) } else { format!("{}\n", c) });
        }
    }
    Out { out, err, code: if any { 0 } else { 1 } }
}

/// Caminha recursivamente (com limite de profundidade). Retorna (caminho absoluto, tipo).
pub fn walk(os: &Os, sim: &Sim, root: &str, max_depth: usize) -> Vec<(String, Kind)> {
    let mut out = vec![];
    fn rec(os: &Os, sim: &Sim, dir: &str, depth: usize, max: usize, out: &mut Vec<(String, Kind)>) {
        if depth > max || dir == "/proc" || dir == "/dev" || dir == "/usr/bin" || dir == "/bin" {
            return;
        }
        if let Ok(entries) = os.list(sim, dir) {
            for (n, k) in entries {
                let p = if dir == "/" { format!("/{}", n) } else { format!("{}/{}", dir, n) };
                let isd = k.is_dir();
                out.push((p.clone(), k));
                if isd {
                    rec(os, sim, &p, depth + 1, max, out);
                }
            }
        }
    }
    rec(os, sim, root, 0, max_depth, &mut out);
    out
}

fn wc(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String], stdin: &str) -> Out {
    let (s, _, paths) = opts(args);
    let (inputs, err) = input_or_files(os, sim, sh, "wc", &paths, stdin);
    let mut out = String::new();
    for (name, d) in inputs {
        let l = d.lines().count();
        let w = d.split_whitespace().count();
        let c = d.len();
        let fields: Vec<String> = if s.is_empty() {
            vec![l.to_string(), w.to_string(), c.to_string()]
        } else {
            let mut f = vec![];
            if s.contains('l') { f.push(l.to_string()) }
            if s.contains('w') { f.push(w.to_string()) }
            if s.contains('c') { f.push(c.to_string()) }
            f
        };
        out.push_str(&format!("{} {}\n", fields.iter().map(|x| format!("{:>7}", x)).collect::<Vec<_>>().join(" "), name).trim_end().to_string());
        out.push('\n');
    }
    Out { out, code: if err.is_empty() { 0 } else { 1 }, err }
}

fn touch(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String]) -> Out {
    let (_, _, paths) = opts(args);
    let mut err = String::new();
    for p in &paths {
        let a = abs(sh, p);
        if os.kind(sim, &a).is_none() {
            if let Err(e) = os.write(sim, &a, "", false, "") {
                err.push_str(&format!("touch: não foi possível tocar '{}': {}\n", p, e));
            }
        }
    }
    Out { out: String::new(), code: if err.is_empty() { 0 } else { 1 }, err }
}

fn mkdir(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String]) -> Out {
    let (s, _, paths) = opts(args);
    let mut err = String::new();
    for p in &paths {
        let a = abs(sh, p);
        if os.mount_for(&a).is_some() || a.starts_with("/proc") || a.starts_with("/dev") {
            err.push_str(&format!("mkdir: não foi possível criar o diretório '{}': Sistema de arquivos somente para leitura\n", p));
            continue;
        }
        if !(a.starts_with("/home/perito") || a.starts_with("/tmp") || a.starts_with("/mnt") || a.starts_with("/media")) {
            err.push_str(&format!("mkdir: não foi possível criar o diretório '{}': Permissão negada\n", p));
            continue;
        }
        if os.kind(sim, &a).is_some() {
            if !s.contains('p') {
                err.push_str(&format!("mkdir: não foi possível criar o diretório '{}': Arquivo existe\n", p));
            }
            continue;
        }
        if !s.contains('p') && !k_is_dir(os, sim, &dirname(&a)) {
            err.push_str(&format!("mkdir: não foi possível criar o diretório '{}': Arquivo ou diretório inexistente\n", p));
            continue;
        }
        os.mkdir_p(&a, sim.clock);
    }
    Out { out: String::new(), code: if err.is_empty() { 0 } else { 1 }, err }
}

fn rm(os: &mut Os, sim: &mut Sim, sh: &Shell, cmd: &str, args: &[String]) -> Out {
    let (s, _, paths) = opts(args);
    let mut err = String::new();
    for p in &paths {
        let a = abs(sh, p);
        let Some(k) = os.kind(sim, &a) else {
            if !s.contains('f') {
                err.push_str(&format!("{}: não foi possível remover '{}': Arquivo ou diretório inexistente\n", cmd, p));
            }
            continue;
        };
        if os.mount_for(&a).is_some() || matches!(k, Kind::Dev { .. } | Kind::Proc) {
            err.push_str(&format!("{}: não foi possível remover '{}': Sistema de arquivos somente para leitura\n", cmd, p));
            continue;
        }
        if matches!(k, Kind::Image { .. }) {
            err.push_str(&format!("{}: não foi possível remover '{}': Operação não permitida (imagem forense protegida)\n", cmd, p));
            continue;
        }
        if !a.starts_with("/home/perito/") && !a.starts_with("/tmp/") && !a.starts_with("/mnt/") {
            err.push_str(&format!("{}: não foi possível remover '{}': Permissão negada\n", cmd, p));
            continue;
        }
        if k.is_dir() {
            let empty = os.list(sim, &a).map(|e| e.is_empty()).unwrap_or(false);
            if cmd == "rmdir" && !empty {
                err.push_str(&format!("rmdir: falhou em remover '{}': Diretório não vazio\n", p));
                continue;
            }
            if cmd == "rm" && !s.contains('r') && !s.contains('R') {
                err.push_str(&format!("rm: não foi possível remover '{}': É um diretório\n", p));
                continue;
            }
            if os.mounts.iter().any(|m| m.point == a || m.point.starts_with(&format!("{}/", a))) {
                err.push_str(&format!("{}: não foi possível remover '{}': Dispositivo ou recurso está ocupado\n", cmd, p));
                continue;
            }
        }
        os.remove(&a);
    }
    Out { out: String::new(), code: if err.is_empty() { 0 } else { 1 }, err }
}

fn cp_mv(os: &mut Os, sim: &mut Sim, sh: &Shell, cmd: &str, args: &[String]) -> Out {
    let (s, _, pos) = opts(args);
    if pos.len() < 2 {
        return Out::fail(cmd, "falta o operando arquivo de destino");
    }
    let dst = pos.last().unwrap();
    let dst_abs = abs(sh, dst);
    let dst_is_dir = k_is_dir(os, sim, &dst_abs);
    let mut err = String::new();
    for src in &pos[..pos.len() - 1] {
        let sa = abs(sh, src);
        let Some(k) = os.kind(sim, &sa) else {
            err.push_str(&format!("{}: não foi possível obter estado de '{}': Arquivo ou diretório inexistente\n", cmd, src));
            continue;
        };
        let target = if dst_is_dir { format!("{}/{}", dst_abs.trim_end_matches('/'), basename(&sa)) } else { dst_abs.clone() };
        if k.is_dir() {
            if !s.contains('r') && !s.contains('R') && !s.contains('a') {
                err.push_str(&format!("{}: -r não especificado; omitindo o diretório '{}'\n", cmd, src));
                continue;
            }
            // cópia recursiva (somente arquivos regulares)
            for (p, k2) in walk(os, sim, &sa, 10) {
                let rel = &p[sa.len()..];
                let t = format!("{}{}", target, rel);
                if k2.is_dir() {
                    os.mkdir_p(&t, sim.clock);
                } else if !matches!(k2, Kind::MFile { entry: None, .. }) {
                    if let Ok(d) = os.read(sim, &p) {
                        let origin = format!("cópia de {}", p);
                        if let Err(e) = os.write(sim, &t, &d, false, &origin) {
                            err.push_str(&format!("{}: não foi possível criar '{}': {}\n", cmd, t, e));
                        }
                    }
                }
            }
        } else {
            if matches!(k, Kind::Image { .. }) {
                err.push_str(&format!("{}: '{}': imagens forenses não são copiadas com cp (use 'pericia imagem <mídia>' para gerar produtos com hash e custódia)\n", cmd, src));
                continue;
            }
            match os.read(sim, &sa) {
                Ok(d) => {
                    let origin = format!("cópia de {}", sa);
                    if let Err(e) = os.write(sim, &target, &d, false, &origin) {
                        err.push_str(&format!("{}: não foi possível criar arquivo comum '{}': {}\n", cmd, dst, e));
                        continue;
                    }
                }
                Err(e) => {
                    err.push_str(&format!("{}: não foi possível abrir '{}' para leitura: {}\n", cmd, src, e));
                    continue;
                }
            }
        }
        if cmd == "mv" {
            if os.mount_for(&sa).is_some() || matches!(k, Kind::Dev { .. }) {
                err.push_str(&format!("mv: não foi possível remover '{}': Sistema de arquivos somente para leitura\n", src));
            } else {
                os.remove(&sa);
            }
        }
    }
    Out { out: String::new(), code: if err.is_empty() { 0 } else { 1 }, err }
}

pub fn describe_type(os: &mut Os, sim: &Sim, a: &str, k: &Kind) -> String {
    let _ = os;
    match k {
        Kind::Dir | Kind::MDir { .. } => "directory".into(),
        Kind::Dev { media: Some(m), .. } => format!("block special (evidência {})", m),
        Kind::Dev { .. } => "character special".into(),
        Kind::Proc => "empty".into(),
        Kind::Image { media, .. } => {
            let parts = sim.os_partitions(media);
            let fs: Vec<String> = parts.iter().map(|p| p.fs.clone()).collect();
            format!("DOS/MBR boot sector; partition 1 : ID=0x7, active ({}) — imagem forense bruta de {}", fs.join(", "), media)
        }
        Kind::MFile { entry: None, .. } => "data".into(),
        Kind::MFile { entry: Some(e), .. } => {
            if e.container {
                "data (alta entropia — possível contêiner cifrado)".into()
            } else if e.composite {
                "Zip archive data, at least v2.0 to extract".into()
            } else {
                match e.ftype {
                    crate::model::FileType::Documento => "Microsoft Word 2007+".into(),
                    crate::model::FileType::Planilha => "Microsoft Excel 2007+".into(),
                    crate::model::FileType::Imagem => "JPEG image data, JFIF standard 1.01, Exif".into(),
                    crate::model::FileType::Audio => "Audio file with ID3 version 2.4.0".into(),
                    crate::model::FileType::BancoDados => "CSV text".into(),
                    crate::model::FileType::Email => "SMTP mail, Unicode text, UTF-8 text".into(),
                    _ => "Unicode text, UTF-8 text".into(),
                }
            }
        }
        Kind::File { .. } => {
            if a.ends_with(".eml") {
                "SMTP mail, Unicode text, UTF-8 text".into()
            } else if a.ends_with(".pdf") {
                "PDF document".into()
            } else {
                "Unicode text, UTF-8 text".into()
            }
        }
    }
}

fn file(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String]) -> Out {
    let (_, _, paths) = opts(args);
    let mut out = String::new();
    for p in &paths {
        let a = abs(sh, p);
        match os.kind(sim, &a) {
            Some(k) => out.push_str(&format!("{}: {}\n", p, describe_type(os, sim, &a, &k))),
            None => out.push_str(&format!("{}: cannot open `{}' (No such file or directory)\n", p, p)),
        }
    }
    Out::ok(out)
}

fn stat(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String]) -> Out {
    let (_, _, paths) = opts(args);
    let mut out = String::new();
    let mut err = String::new();
    for p in &paths {
        let a = abs(sh, p);
        let Some(k) = os.kind(sim, &a) else {
            err.push_str(&format!("stat: não foi possível obter estado de '{}': Arquivo ou diretório inexistente\n", p));
            continue;
        };
        let (sz, mt) = size_mtime(os, &k);
        let (ct, at, tz) = match &k {
            Kind::MFile { entry: Some(e), mount, .. } => {
                let tz = os.mount_view(sim, *mount).map(|v| v.tz_offset).unwrap_or(BRT);
                (e.ctime, e.atime, tz)
            }
            _ => (mt, mt, BRT),
        };
        let tipo = match &k {
            Kind::Dir | Kind::MDir { .. } => "diretório",
            Kind::Dev { .. } => "arquivo especial de bloco",
            _ => "arquivo comum",
        };
        out.push_str(&format!("  Arquivo: {}\n  Tamanho: {:<12} Blocos: {:<8} bloco de E/S: 4096   {}\nAcesso: ({}) Uid: ({}) \n", p, sz, sz.div_ceil(512), tipo, perms(&k), owner(&k)));
        out.push_str(&format!("Acesso: {}\nModificação: {}\nMudança: {}\n", fmt_dtz(at, tz), fmt_dtz(mt, tz), fmt_dtz(ct, tz)));
        if tz != BRT {
            out.push_str(&format!("ATENÇÃO: horários exibidos no fuso do volume examinado ({}).\n", fmt_offset(tz)));
        }
        if let Some(Node::File(f)) = os.local_node(&a) {
            if !f.origin.is_empty() {
                out.push_str(&format!("Origem (produto derivado): {}\n", f.origin));
            }
        }
    }
    Out { out, code: if err.is_empty() { 0 } else { 1 }, err }
}

fn find(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String]) -> Out {
    let mut root = ".".to_string();
    let mut name: Option<(String, bool)> = None;
    let mut ftype: Option<char> = None;
    let mut i = 0;
    if let Some(a) = args.first() {
        if !a.starts_with('-') {
            root = a.clone();
            i = 1;
        }
    }
    while i < args.len() {
        match args[i].as_str() {
            "-name" | "-iname" if i + 1 < args.len() => {
                name = Some((args[i + 1].clone(), args[i] == "-iname"));
                i += 2;
            }
            "-type" if i + 1 < args.len() => {
                ftype = args[i + 1].chars().next();
                i += 2;
            }
            x => return Out::fail("find", format!("predicado desconhecido '{}'", x)),
        }
    }
    let ra = abs(sh, &root);
    if os.kind(sim, &ra).is_none() {
        return Out::fail("find", format!("'{}': Arquivo ou diretório inexistente", root));
    }
    let mut out = String::new();
    for (p, k) in walk(os, sim, &ra, 14) {
        let n = basename(&p);
        if let Some((pat, ci)) = &name {
            let ok = if *ci { glob_match(&fold(pat), &fold(n)) } else { glob_match(pat, n) };
            if !ok {
                continue;
            }
        }
        if let Some(t) = ftype {
            if (t == 'd') != k.is_dir() {
                continue;
            }
        }
        let shown = if root == "." { format!(".{}", &p[ra.len()..]) } else { format!("{}{}", root.trim_end_matches('/'), &p[ra.len()..]) };
        out.push_str(&shown);
        out.push('\n');
    }
    Out::ok(out)
}

fn sort_uniq(os: &mut Os, sim: &mut Sim, sh: &Shell, cmd: &str, args: &[String], stdin: &str) -> Out {
    let (s, _, paths) = opts(args);
    let (inputs, err) = input_or_files(os, sim, sh, cmd, &paths, stdin);
    let mut lines: Vec<String> = inputs.iter().flat_map(|(_, d)| d.lines().map(|l| l.to_string()).collect::<Vec<_>>()).collect();
    let mut out = String::new();
    if cmd == "sort" {
        lines.sort();
        if s.contains('r') {
            lines.reverse();
        }
        if s.contains('u') {
            lines.dedup();
        }
        for l in lines {
            out.push_str(&l);
            out.push('\n');
        }
    } else {
        let mut prev: Option<String> = None;
        let mut n = 0;
        let emit = |out: &mut String, l: &str, n: usize| {
            if s.contains('c') {
                out.push_str(&format!("{:>7} {}\n", n, l));
            } else {
                out.push_str(&format!("{}\n", l));
            }
        };
        for l in lines {
            if prev.as_deref() == Some(l.as_str()) {
                n += 1;
            } else {
                if let Some(p) = &prev {
                    emit(&mut out, p, n);
                }
                prev = Some(l);
                n = 1;
            }
        }
        if let Some(p) = &prev {
            emit(&mut out, p, n);
        }
    }
    Out { out, code: if err.is_empty() { 0 } else { 1 }, err }
}

fn tree(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String]) -> Out {
    let (_, _, p) = opts(args);
    let root = p.first().cloned().unwrap_or_else(|| ".".into());
    let ra = abs(sh, &root);
    let mut out = format!("{}\n", root);
    let (mut d, mut f) = (0, 0);
    for (path, k) in walk(os, sim, &ra, 6) {
        if matches!(k, Kind::MFile { entry: None, .. }) {
            continue;
        }
        let rel = &path[ra.len()..];
        let depth = rel.matches('/').count();
        out.push_str(&format!("{}└── {}\n", "│   ".repeat(depth.saturating_sub(1)), basename(&path)));
        if k.is_dir() { d += 1 } else { f += 1 }
    }
    out.push_str(&format!("\n{} diretórios, {} arquivos\n", d, f));
    Out::ok(out)
}

// ------------------------------------------------------------------ discos e montagem

fn lsblk(os: &mut Os, sim: &mut Sim) -> Out {
    let mut out = String::from("NAME        MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS\n");
    for (k, (name, media, part, size, wb)) in os.block_devices(sim).into_iter().enumerate() {
        let is_part = part.is_some() || name == "sda1" || name == "sda2";
        let mp = if name == "sda1" {
            "/".to_string()
        } else if name == "sda2" {
            "/home".to_string()
        } else {
            os.mounts.iter().filter(|m| m.source == format!("/dev/{}", name)).map(|m| m.point.clone()).collect::<Vec<_>>().join(",")
        };
        let ro = if media.is_some() && wb { "1" } else { "0" };
        let shown = if is_part { format!("├─{}", name) } else { name.clone() };
        out.push_str(&format!("{:<11} {:>3}:{:<3} {}  {:>5}  {} {:<4} {}\n", shown, 8, k * 16, if media.is_some() { 1 } else { 0 }, hsize(size), ro, if is_part { "part" } else { "disk" }, mp));
    }
    for (i, m) in os.mounts.iter().enumerate() {
        if m.source.starts_with("/dev/loop") {
            out.push_str(&format!("loop{:<7} {:>3}:{:<3} 0      -  1 loop {}\n", i, 7, i, m.point));
        }
    }
    if sim.attached_media().is_empty() {
        out.push_str("\n(nenhuma mídia de evidência conectada — conecte via bloqueador na Área de Hardware)\n");
    }
    Out::ok(out)
}

fn mount(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String]) -> Out {
    // mount [-o ro,loop,part=N] [-t tipo] ORIGEM DESTINO
    let mut options = String::new();
    let mut pos = vec![];
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" if i + 1 < args.len() => {
                options = args[i + 1].clone();
                i += 2;
            }
            "-t" if i + 1 < args.len() => i += 2,
            "-r" => {
                options.push_str(",ro");
                i += 1;
            }
            a => {
                pos.push(a.to_string());
                i += 1;
            }
        }
    }
    if pos.is_empty() {
        let mut out = os.proc(sim, "mounts");
        out = out.lines().map(|l| {
            let f: Vec<&str> = l.split(' ').collect();
            format!("{} on {} type {} ({})\n", f[0], f[1], f[2], f[3])
        }).collect();
        return Out::ok(out);
    }
    if pos.len() < 2 {
        return Out::fail("mount", "uso: mount [-o ro,loop,part=N] ORIGEM DESTINO").code(1);
    }
    let src = abs(sh, &pos[0]);
    let dst = abs(sh, &pos[1]);
    if !k_is_dir(os, sim, &dst) {
        return Out::fail("mount", format!("{}: o ponto de montagem não existe.", pos[1])).code(32);
    }
    if os.mounts.iter().any(|m| m.point == dst) {
        return Out::fail("mount", format!("{}: já está montado ou ponto de montagem ocupado.", pos[1])).code(32);
    }
    let ro = options.split(',').any(|o| o == "ro");
    let part_opt: Option<u32> = options.split(',').find_map(|o| o.strip_prefix("part=").and_then(|v| v.parse().ok()));
    let kind = os.kind(sim, &src);
    let (media, part_id, original, source, via_blocker) = match kind {
        Some(Kind::Image { media, .. }) => {
            let parts = sim.os_partitions(&media);
            let chosen = match part_opt {
                Some(n) => parts.iter().find(|p| p.index == n),
                None if parts.len() == 1 => parts.first(),
                None => {
                    let mut msg = format!("{}: a imagem tem {} partições; indique qual com -o ro,loop,part=N:\n", pos[0], parts.len());
                    for p in &parts {
                        msg.push_str(&format!("   part={}  {}  {}  {}\n", p.index, p.fs, p.label, hsize(p.size)));
                    }
                    return Out::fail("mount", msg).code(32);
                }
            };
            let Some(p) = chosen else { return Out::fail("mount", "partição inexistente na imagem").code(32) };
            (media, p.id.clone(), false, format!("/dev/loop{}", os.mounts.len()), true)
        }
        Some(Kind::Dev { media: Some(media), part, .. }) => {
            let Some(part) = part else {
                return Out::fail("mount", format!("{}: tipo de sistema de arquivos errado — é o disco inteiro; monte uma partição (ex.: {}1).", pos[0], pos[0])).code(32);
            };
            let wb = sim.hashes.contains_key(&format!("wb:{}", media));
            (media, part, true, src.clone(), wb)
        }
        Some(_) => return Out::fail("mount", format!("{}: não é um dispositivo de bloco nem uma imagem reconhecida.", pos[0])).code(32),
        None => return Out::fail("mount", format!("{}: dispositivo especial não existe.", pos[0])).code(32),
    };
    // volume cifrado?
    let view = sim.os_partitions(&media).into_iter().find(|p| p.id == part_id);
    if let Some(v) = &view {
        if let Some(scheme) = &v.encrypted {
            if !sim.decrypted.contains(&part_id) {
                return Out::fail("mount", format!("{}: tipo de sistema de arquivos desconhecido: volume cifrado ({}). Use pericia-decifrar.", pos[0], scheme)).code(32);
            }
        }
    }
    let mut warn = String::new();
    if original {
        sim.os_mount_original(&media, via_blocker);
        if via_blocker && !ro {
            warn.push_str(&format!("mount: {}: AVISO: origem protegida contra gravação, montada somente para leitura.\n", pos[1]));
        }
        warn.push_str("AVISO: você montou a MÍDIA ORIGINAL. O POP 5.01 manda examinar a cópia (imagem forense). Registrado na auditoria.\n");
    } else {
        sim.exec(&format!("fs montar {}", media));
        if !ro {
            warn.push_str("mount: imagem forense montada somente para leitura (política do laboratório).\n");
        }
    }
    let fstype = view.map(|v| v.fs.to_lowercase()).unwrap_or_default();
    os.mounts.push(super::Mount { point: dst.clone(), source, media: media.clone(), part: part_id, fstype, original });
    sim.rec_pub("montar", &format!("{} em {}", pos[0], dst), if original { "original" } else { "imagem (somente leitura)" });
    Out { out: String::new(), err: warn, code: 0 }
}

fn umount(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String]) -> Out {
    let Some(t) = args.first() else { return Out::fail("umount", "uso: umount PONTO") };
    let a = abs(sh, t);
    if sh.cwd == a || sh.cwd.starts_with(&format!("{}/", a)) {
        return Out::fail("umount", format!("{}: o alvo está ocupado.", t)).code(32);
    }
    let before = os.mounts.len();
    os.mounts.retain(|m| m.point != a && m.source != a);
    if os.mounts.len() == before {
        return Out::fail("umount", format!("{}: não montado.", t)).code(32);
    }
    sim.rec_pub("desmontar", &a, "ok");
    Out::ok("")
}

fn df(os: &mut Os, sim: &mut Sim) -> Out {
    let mut out = String::from("Sist. Arq.      Tam. Usado Disp. Uso% Montado em\n/dev/sda1        94G   18G   71G  21% /\n/dev/sda2       384G   41G  324G  12% /home\n");
    for m in &os.mounts {
        let size = sim.os_partitions(&m.media).iter().find(|p| p.id == m.part).map(|p| p.size).unwrap_or(0);
        out.push_str(&format!("{:<15} {:>5} {:>5} {:>5} {:>4} {}\n", m.source, hsize(size), hsize(size * 6 / 10), hsize(size * 4 / 10), "60%", m.point));
    }
    Out::ok(out)
}

fn dmesg(os: &mut Os, sim: &mut Sim) -> Out {
    let mut out = String::from("[    0.000000] Linux version 6.1.0-28-amd64\n[    2.104411] EXT4-fs (sda1): mounted filesystem with ordered data mode\n");
    let mut t = 812.0;
    for (name, media, part, size, wb) in os.block_devices(sim) {
        if let (Some(m), None) = (media, part) {
            t += 3.7;
            out.push_str(&format!("[{:>12.6}] usb 2-1: new SuperSpeed USB device ({})\n", t, if wb { "via bloqueador de escrita" } else { "conexão direta" }));
            out.push_str(&format!("[{:>12.6}] sd 6:0:0:0: [{}] {} 512-byte logical blocks ({}) — evidência {}\n", t + 0.2, name, size / 512, fmt_bytes(size), m));
            out.push_str(&format!("[{:>12.6}] sd 6:0:0:0: [{}] Write Protect is {}\n", t + 0.3, name, if wb { "on" } else { "off" }));
        }
    }
    Out::ok(out)
}

fn sha256sum(os: &mut Os, sim: &mut Sim, sh: &Shell, args: &[String], stdin: &str) -> Out {
    let (_, _, paths) = opts(args);
    if paths.is_empty() {
        return Out::ok(format!("{}  -\n", crate::sha256::sha256(stdin)));
    }
    let mut out = String::new();
    let mut err = String::new();
    for p in &paths {
        let a = abs(sh, p);
        match os.kind(sim, &a) {
            Some(Kind::Image { media, .. }) => {
                sim.os_tick(8);
                let h = crate::vfs::media_hash(sim.case.media_by_id(&media).unwrap());
                // conferência: hash da imagem comparado com o do original/aquisição
                if sim.hashes.contains_key(&format!("hash:{}", media)) || sim.hashes.contains_key(&format!("img:{}", media)) {
                    sim.exec(&format!("verificar-hash {}", media));
                }
                out.push_str(&format!("{}  {}\n", h, p));
            }
            Some(Kind::Dev { media: Some(media), .. }) => {
                let r = sim.exec(&format!("hash {}", media));
                let h = r.text.lines().last().unwrap_or("").trim().to_string();
                if sim.imaged.contains(&media) {
                    sim.exec(&format!("verificar-hash {}", media));
                }
                out.push_str(&format!("{}  {}\n", h, p));
            }
            Some(Kind::MFile { mount, entry: Some(e), .. }) => {
                let m = os.mounts[mount].clone();
                match sim.os_file_hash(&m.media, &m.part, &e.path) {
                    Ok(h) => out.push_str(&format!("{}  {}\n", h, p)),
                    Err(x) => err.push_str(&format!("sha256sum: {}: {}\n", p, x)),
                }
            }
            Some(k) if k.is_dir() => err.push_str(&format!("sha256sum: {}: É um diretório\n", p)),
            Some(_) => match os.read(sim, &a) {
                Ok(d) => out.push_str(&format!("{}  {}\n", crate::sha256::sha256(&d), p)),
                Err(e) => err.push_str(&format!("sha256sum: {}: {}\n", p, e)),
            },
            None => err.push_str(&format!("sha256sum: {}: Arquivo ou diretório inexistente\n", p)),
        }
    }
    Out { out, code: if err.is_empty() { 0 } else { 1 }, err }
}

pub fn distro_line() -> String {
    format!("{} {}", DISTRO, DISTRO_VERSION)
}

#[allow(dead_code)]
fn unused(_n: Node) {
    let _ = norm("/");
}
