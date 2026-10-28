//! Exploração: sequências aleatórias de comandos em todos os módulos — procura pânicos,
//! auditoria inválida e não determinismo.
use pericia_core::casegen::generate;
use pericia_core::engine::Sim;
use pericia_core::model::*;
use pericia_core::rng::Rng;

const VERBS: &[&str] = &["fotografar {i}", "lacre conferir {i}", "lacre aplicar {i}", "deslacrar {i}", "abrir {i}", "componente remover {i} dissipador",
 "componente remover {i} tampa", "midia listar {i}", "midia remover {i} {s}", "bloqueio {m}", "imagem {m}", "sanitizar {m}", "clone {m} {m}", "hash {m}",
 "verificar-hash {m}", "energia desligar {i}", "energia ligar {i}", "memoria {i}", "volume copiar {i}", "rede isolar {i}", "inspecionar {i}", "inspecionar local",
 "isolar {i} faraday", "isolar {i} aluminio", "airplane {i}", "sim remover {i} 1", "alarmes {i}", "carregar {i}", "desbloquear {i} 1234", "extrair {i}",
 "extrair {i} fs", "extrair {i} logica", "senha", "camera cobrir {i}", "solicitar-senhas {i}", "imei verificar {i}", "tela {i} mensagens", "tela {i} foto",
 "embalar {i}", "etiquetar {i} x", "fs montar {m}", "fs ls {m} /", "fs buscar {m} a", "fs meta {m} /", "recuperar {m} x", "expandir {m} x", "decifrar {m} x",
 "ocr {m} x", "dicionario {i}", "dns x.sim", "whois x.sim", "navegar https://x.sim/", "preservar https://x.sim/", "email cabecalho x", "conexao anonima", "bios {i}", "bios ordem {i} forense", "nuvem", "nuvem solicitar x", "nuvem acessar x y", "nuvem ls", "nuvem cat x", "decifrar {m} x",
 "anotar teste", "achei x", "laudo auto", "laudo conclusao teste", "finalizar", "", "   ", "item", "item zzz", "fotografar", "lacre", "|", "laudo", "fs", "tela", "tela {i}",
 "extrair", "sim remover", "sim remover {i} 9", "clone", "imagem", "hash", "achei", "anotar", "ajuda", "caso", "itens", "tempo", "notas", "custodia", "auditoria", "progresso"];

fn main() {
    let mut problems = 0;
    let mut runs = 0;
    for m in ModuleId::ALL {
        for d in 1..=4u8 {
            for seed in 0..25u32 {
                let case = generate(m, d, seed);
                let items: Vec<String> = case.items.iter().map(|i| i.id.to_lowercase()).collect();
                let media: Vec<String> = case.media.iter().map(|x| x.id.to_lowercase()).collect();
                let mut r = Rng::new(seed * 7919 + d as u32);
                let mut cmds = vec![];
                for _ in 0..60 {
                    let v = VERBS[r.int(0, VERBS.len() as i64 - 1) as usize];
                    let it = items[r.int(0, items.len() as i64 - 1) as usize].clone();
                    let md = media.get(r.int(0, media.len().max(1) as i64 - 1) as usize).cloned().unwrap_or("m-01".into());
                    cmds.push(v.replace("{i}", &it).replace("{m}", &md).replace("{s}", "s1"));
                }
                let run = |cmds: &[String]| {
                    let mut s = Sim::new(generate(m, d, seed));
                    for c in cmds { s.exec(c); }
                    s
                };
                let res = std::panic::catch_unwind(|| run(&cmds));
                runs += 1;
                match res {
                    Err(_) => { problems += 1; eprintln!("PÂNICO {:?} D{} s{}: {:?}", m, d, seed, cmds); }
                    Ok(s) => {
                        if s.audit.verify().is_err() { problems += 1; eprintln!("AUDITORIA {:?} D{} s{}", m, d, seed); }
                        let s2 = run(&cmds);
                        if s2.audit.last_hash() != s.audit.last_hash() { problems += 1; eprintln!("NÃO DETERMINISTA {:?} D{} s{}", m, d, seed); }
                    }
                }
            }
        }
    }
    println!("execuções={} problemas={}", runs, problems);
}
