use pericia_core::casegen::{case_id, generate};
use pericia_core::model::*;
fn main() {
    for m in ModuleId::ALL { for d in 1..=4u8 { for s in 1..30u32 {
        let c = generate(m, d, s);
        let ks: Vec<String> = c.items.iter().filter(|i| !i.hidden).map(|i| format!("{:?}", i.kind)).collect();
        if ks.iter().any(|k| k == "Pendrive" || k == "Ssd") { println!("{} {:?}", case_id(m, d, s), ks); return; }
    }}}
}
