//! Gera um laudo de exemplo jogando um caso 5.01 do início ao fim (usado na validação).
use pericia_core::{casegen::generate, engine::Sim, model::ModuleId, pdf, report};
fn main() {
    let out = std::env::args().nth(1).unwrap_or("laudo_exemplo".into());
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    let kw = s.case.keyword.clone();
    for c in ["fotografar item-01", "lacre conferir item-01", "deslacrar item-01", "bloqueio m-01", "imagem m-01", "verificar-hash m-01", "fs montar m-01"] { s.exec(c); }
    let user = pericia_core::content::login(s.case.suspect());
    s.exec(&format!("recuperar m-01 /Users/{}/Documents/entrega_final.docx", user));
    s.exec(&format!("achei {}", kw));
    s.exec("laudo auto");
    s.exec("laudo conclusao Na imagem forense da mídia M-01 (hash verificado) foi recuperado arquivo apagado contendo a expressão de interesse. A presença do arquivo na conta do usuário é fato; a autoria do conteúdo depende de outras evidências.");
    s.exec("lacre aplicar item-01");
    println!("{}", s.exec("finalizar").text);
    std::fs::write(format!("{}.pdf", out), pdf::document_pdf(&s)).unwrap();
    std::fs::write(format!("{}.md", out), report::markdown(&s)).unwrap();
}
