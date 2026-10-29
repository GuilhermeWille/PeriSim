use pericia_core::casegen::generate;
use pericia_core::engine::Sim;
use pericia_core::model::*;
fn main() {
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    for c in ["imagem", "imagem m-99", "hash", "bloqueio", "fotografar", "fotografar item-99", "lacre", "lacre abrir item-01", "deslacrar", "abrir", "componente", "componente remover item-01",
              "midia", "midia remover", "clone", "clone m-01", "sanitizar", "verificar-hash", "energia", "energia ligar", "memoria", "volume", "rede", "isolar", "isolar item-01", "isolar item-01 papel",
              "sim", "extrair", "extrair item-01", "desbloquear item-01", "fs", "fs montar", "fs ls", "recuperar", "expandir", "decifrar", "ocr", "dicionario", "dns", "whois", "navegar", "preservar",
              "email", "conexao x", "laudo", "laudo xyz", "etiquetar", "embalar", "inspecionar", "tela", "camera", "imei", "item", "IMAGEM M-01", "Fotografar Item-01"] {
        let o = s.exec(c);
        println!("{:<28} {} {}", c, if o.ok { "ok " } else { "ERR" }, o.text.lines().next().unwrap_or(""));
    }
}
