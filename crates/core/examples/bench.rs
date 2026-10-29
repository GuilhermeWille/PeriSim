//! Benchmark do motor (sem interface): geração de casos, hash de mídia, comandos,
//! PDF e replay. Uso: cargo run --release -p pericia_core --example bench
use pericia_core::{casegen::generate, engine::Sim, model::ModuleId, pdf, vfs};
use std::time::Instant;

fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1000.0 }

fn main() {
    println!("| Medida | Pequeno (D1) | Médio (D2/D3) | Grande (D4) |\n|---|---|---|---|");
    let mut gen = vec![]; let mut size = vec![]; let mut hash = vec![]; let mut files = vec![];
    for d in [1u8, 3, 4] {
        let n = 200;
        let t = Instant::now();
        let mut bytes = 0usize; let mut nf = 0u64;
        for seed in 0..n {
            for m in ModuleId::ALL {
                let c = generate(m, d, seed);
                bytes += serde_json::to_string(&c).unwrap().len();
                nf += c.media.iter().flat_map(|m| m.partitions.iter()).map(vfs::total_files).sum::<u64>();
            }
        }
        gen.push(format!("{:.3} ms", ms(t) / (n as f64 * 5.0)));
        size.push(format!("{:.1} KB", bytes as f64 / (n as f64 * 5.0) / 1000.0));
        files.push(format!("{:.0}", nf as f64 / (n as f64 * 5.0)));
        let c = generate(ModuleId::M503, d, 1);
        let t = Instant::now();
        for _ in 0..20 { for m in &c.media { let _ = vfs::media_hash(m); } }
        hash.push(format!("{:.2} ms", ms(t) / 20.0));
    }
    println!("| Geração de caso (média, 5 módulos) | {} | {} | {} |", gen[0], gen[1], gen[2]);
    println!("| Tamanho serializado do caso | {} | {} | {} |", size[0], size[1], size[2]);
    println!("| Arquivos representados (reais + sistema, sob demanda) | {} | {} | {} |", files[0], files[1], files[2]);
    println!("| Hash de todas as mídias (caso 5.03) | {} | {} | {} |", hash[0], hash[1], hash[2]);

    // fluxo completo + PDF + replay
    let cmds = ["fotografar item-01", "rede isolar item-01", "memoria item-01", "volume copiar item-01", "energia desligar item-01",
        "inspecionar local", "bloqueio m-01", "imagem m-01", "verificar-hash m-01", "fs montar m-01", "fs buscar docx", "laudo auto",
        "laudo conclusao Teste.", "finalizar"];
    let t = Instant::now();
    let mut s = Sim::new(generate(ModuleId::M503, 4, 9));
    for c in cmds { s.exec(c); }
    let per_cmd = ms(t) / cmds.len() as f64;
    let t = Instant::now();
    let pdfb = pdf::document_pdf(&s);
    let tpdf = ms(t);
    let t = Instant::now();
    let mut r = Sim::new(generate(ModuleId::M503, 4, 9));
    for c in cmds { r.exec(c); }
    let trep = ms(t);
    assert_eq!(r.audit.last_hash(), s.audit.last_hash(), "replay deve reproduzir a auditoria");
    println!("\n| Operação | Tempo |\n|---|---|");
    println!("| Comando do terminal (média, fluxo 5.03 D4 com aquisição e hash) | {:.2} ms |", per_cmd);
    println!("| Geração do PDF do laudo ({} bytes) | {:.2} ms |", pdfb.len(), tpdf);
    println!("| Replay completo do salvamento ({} comandos) | {:.2} ms |", cmds.len(), trep);
    println!("| Replay reproduz a mesma auditoria | sim |");
}
