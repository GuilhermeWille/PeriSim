//! Mostra dados de um caso pelo ID (apoio a testes e a instrutores).
use pericia_core::casegen::{generate, parse_case_id};
fn main() {
    let id = std::env::args().nth(1).expect("uso: case_info <ID>");
    let (m, d, s) = parse_case_id(&id).expect("ID inválido");
    let c = generate(m, d, s);
    println!("{}  {}\nexpressão: {}", c.id, c.title, c.keyword);
    if let Some(n) = &c.net { for d in &n.domains { println!("domínio: {}", d.name); } }
    for f in &c.provided { for l in f.content.lines().filter(|l| l.starts_with("X-Originating")) { println!("{}: {}", f.name(), l); } }
    for k in &c.keyword_sources { println!("fonte: {}", k.describe()); }
}
