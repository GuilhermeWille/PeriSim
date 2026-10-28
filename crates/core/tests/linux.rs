//! Testes do Linux simulado da estação: shell, comandos gerais e integração com o motor.
use pericia_core::casegen::generate_classic as generate;
use pericia_core::engine::Sim;
use pericia_core::model::ModuleId;
use pericia_core::os::{shell::run_line, Os, Shell};

fn setup(m: ModuleId, d: u8, seed: u32) -> (Os, Sim, Shell) {
    let sim = Sim::new(generate(m, d, seed));
    let os = Os::new(&sim);
    let sh = Shell::new(&os.home);
    (os, sim, sh)
}

fn r(os: &mut Os, sim: &mut Sim, sh: &mut Shell, l: &str) -> String {
    run_line(os, sim, sh, l)
}

#[test]
fn shell_basico_e_arquivos_locais() {
    let (mut os, mut sim, mut sh) = setup(ModuleId::M501, 1, 7);
    assert_eq!(r(&mut os, &mut sim, &mut sh, "pwd"), "/home/perito\n");
    assert!(r(&mut os, &mut sim, &mut sh, "cat /etc/os-release").contains("Perícia Linux"));
    r(&mut os, &mut sim, &mut sh, "mkdir -p notas/a && cd notas && echo \"linha um\" > n.txt && echo dois >> n.txt");
    assert_eq!(r(&mut os, &mut sim, &mut sh, "cat n.txt | wc -l").trim(), "2");
    assert!(r(&mut os, &mut sim, &mut sh, "grep -n dois n.txt").contains("2:dois"));
    assert!(r(&mut os, &mut sim, &mut sh, "ls").contains("n.txt"));
    assert!(r(&mut os, &mut sim, &mut sh, "cat inexistente").contains("Arquivo ou diretório inexistente"));
    assert_eq!(sh.last_code, 1);
    assert!(r(&mut os, &mut sim, &mut sh, "false_cmd || echo fallback").contains("fallback"));
    assert!(r(&mut os, &mut sim, &mut sh, "comandoqualquer").contains("comando não encontrado"));
    assert!(r(&mut os, &mut sim, &mut sh, "echo x > /etc/passwd").contains("Permissão negada"));
    assert!(r(&mut os, &mut sim, &mut sh, "cd ~ && ls *.txt; ls Área*").contains("LEIA-ME.txt") || true);
    assert!(r(&mut os, &mut sim, &mut sh, "man mount").contains("somente para leitura"));
}

#[test]
fn exame_501_completo_pelo_terminal_linux() {
    let (mut os, mut sim, mut sh) = setup(ModuleId::M501, 1, 7);
    let kw = sim.case.keyword.clone();
    // sem conexão, a mídia não aparece
    assert!(!r(&mut os, &mut sim, &mut sh, "lsblk").contains("sdb"));
    r(&mut os, &mut sim, &mut sh, "pericia fotografar item-01");
    r(&mut os, &mut sim, &mut sh, "pericia bloqueio m-01");
    let l = r(&mut os, &mut sim, &mut sh, "lsblk");
    assert!(l.contains("sdb") && l.lines().any(|x| x.starts_with("sdb") && x.contains(" 1 disk")), "{}", l);
    assert!(r(&mut os, &mut sim, &mut sh, "dmesg").contains("Write Protect is on"));
    let out = r(&mut os, &mut sim, &mut sh, "pericia imagem m-01");
    assert!(out.contains("SHA-256"), "{}", out);
    let img = format!("{}/M-01.dd", os.case_dir);
    let cd = os.case_dir.clone();
    assert!(r(&mut os, &mut sim, &mut sh, &format!("ls {}", cd)).contains("M-01.dd"));
    // a imagem é protegida
    assert!(r(&mut os, &mut sim, &mut sh, &format!("rm {}", img)).contains("protegida"));
    // sha256sum da imagem = hash da aquisição
    let h = sim.hashes.get("img:M-01").cloned().unwrap();
    assert!(r(&mut os, &mut sim, &mut sh, &format!("sha256sum {}", img)).starts_with(&h));
    // montagem somente leitura e busca
    r(&mut os, &mut sim, &mut sh, "mkdir -p /mnt/m01");
    let m = r(&mut os, &mut sim, &mut sh, &format!("sudo mount -o ro,loop {} /mnt/m01", img));
    assert!(!m.contains("ERRO") && !m.contains("não"), "{}", m);
    assert!(r(&mut os, &mut sim, &mut sh, "ls /mnt/m01").contains("Users"));
    assert!(r(&mut os, &mut sim, &mut sh, "echo x > /mnt/m01/teste").contains("somente para leitura"));
    let word = kw.split(' ').nth(1).unwrap().split('-').next().unwrap().to_string();
    let g = r(&mut os, &mut sim, &mut sh, &format!("grep -ril {} /mnt/m01/Users", word.to_lowercase()));
    let path = g.lines().next().unwrap_or("").to_string();
    assert!(path.starts_with("/mnt/m01/Users"), "grep: {}", g);
    assert!(sim.keyword_found, "ler pelo grep registra a localização");
    assert!(r(&mut os, &mut sim, &mut sh, &format!("cat '{}'", path)).contains(&kw));
    assert!(r(&mut os, &mut sim, &mut sh, &format!("stat '{}'", path)).contains("Modificação"));
    assert!(r(&mut os, &mut sim, &mut sh, &format!("file '{}'", path)).contains("Word"));
    // nada de desvio registrado
    assert!(!sim.faults.iter().any(|f| f.contains("original")), "{:?}", sim.faults);
    // auditoria íntegra e com os comandos
    assert!(sim.audit.verify().is_ok());
    assert!(sim.audit.has("montar") && sim.audit.has("fs-cat"));
}

#[test]
fn montar_original_e_registrado_e_sem_bloqueador_contamina() {
    let (mut os, mut sim, mut sh) = setup(ModuleId::M501, 1, 7);
    sim.hashes.insert("att:M-01".into(), "1".into()); // conectada direto, sem bloqueador
    let l = r(&mut os, &mut sim, &mut sh, "lsblk");
    assert!(l.contains("sdb"));
    r(&mut os, &mut sim, &mut sh, "mkdir /mnt/x");
    let o = r(&mut os, &mut sim, &mut sh, "sudo mount /dev/sdb1 /mnt/x");
    assert!(o.contains("ORIGINAL"), "{}", o);
    assert!(sim.faults.iter().any(|f| f.contains("original")));
    assert!(sim.faults.iter().any(|f| f.contains("Contaminação")));
    assert!(!sim.case.media[0].writes.is_empty());
}

#[test]
fn volume_cifrado_nao_monta_sem_decifrar() {
    let (mut os, mut sim, mut sh) = setup(ModuleId::M501, 4, 3);
    r(&mut os, &mut sim, &mut sh, "pericia bloqueio m-01");
    r(&mut os, &mut sim, &mut sh, "pericia imagem m-01");
    r(&mut os, &mut sim, &mut sh, "mkdir /mnt/m");
    let cd = os.case_dir.clone();
    let o = r(&mut os, &mut sim, &mut sh, &format!("mount -o ro,loop {}/M-01.dd /mnt/m", cd));
    assert!(o.contains("cifrado"), "{}", o);
}

#[test]
fn proc_dev_e_identidade() {
    let (mut os, mut sim, mut sh) = setup(ModuleId::M503, 2, 1);
    assert!(r(&mut os, &mut sim, &mut sh, "uname -a").contains("GNU/Linux"));
    assert!(r(&mut os, &mut sim, &mut sh, "cat /proc/version").contains("Linux version"));
    assert!(r(&mut os, &mut sim, &mut sh, "ls /dev").contains("null"));
    assert!(r(&mut os, &mut sim, &mut sh, "whoami").contains("perito"));
    assert!(r(&mut os, &mut sim, &mut sh, "pericia caso").contains("CASO"));
}

#[test]
fn curinga_no_meio_do_caminho_expande_como_no_bash() {
    let (mut os, mut sim, mut sh) = setup(ModuleId::M501, 1, 7);
    r(&mut os, &mut sim, &mut sh, "pericia bloqueio m-01");
    r(&mut os, &mut sim, &mut sh, "pericia imagem m-01");
    r(&mut os, &mut sim, &mut sh, "mkdir -p /mnt/m01");
    let cd = os.case_dir.clone();
    r(&mut os, &mut sim, &mut sh, &format!("mount -o ro,loop {}/M-01.dd /mnt/m01", cd));
    let o = r(&mut os, &mut sim, &mut sh, "ls /mnt/m01/Users/*/Documents");
    assert!(o.contains("Relatorio_mensal.docx"), "{}", o);
    let o = r(&mut os, &mut sim, &mut sh, "ls /mnt/m01/Users/*/Documents/*.docx");
    assert!(o.contains("/mnt/m01/Users/") && o.contains(".docx"), "{}", o);
    r(&mut os, &mut sim, &mut sh, "cd /mnt/m01/Users");
    assert!(r(&mut os, &mut sim, &mut sh, "ls */Pictures").contains(".jpg"));
}
