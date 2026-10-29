use pericia_core::casegen::generate;
use pericia_core::engine::Sim;
use pericia_core::model::*;
fn main() {
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    for c in ["luvas calcar","pulseira colocar","pegar camera","fotografar item-01","pegar oficio","lacre conferir item-01","pegar tesoura","deslacrar item-01","pegar lanterna","inspecionar item-01","pegar chave","desparafusar item-01","desparafusar item-01","abrir item-01","cabo desconectar item-01 sata0 dados","pegar saco","embalar item-01"] {
        let o = s.exec(c); println!("{} {}", if o.ok {"ok "} else {"ERR"}, c);
    }
    for e in &s.audit.entries { println!("  action={:?} detail={:?}", e.action, e.detail); }
}
