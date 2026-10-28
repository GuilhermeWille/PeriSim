use pericia_core::casegen::{case_id, generate};
use pericia_core::model::*;
fn main() {
    for m in ModuleId::ALL {
        for d in 1..=4 {
            if let Some(s) = (1..60).find(|&s| generate(m, d, s).items.iter().any(|i| i.slots.iter().any(|x| x.hidden_by.is_some()))) {
                let c = generate(m, d, s);
                let it = c.items.iter().find(|i| i.slots.iter().any(|x| x.hidden_by.is_some())).unwrap();
                println!("{} {} {:?} itens={:?}", case_id(m, d, s), it.id, c.setting, c.items.iter().map(|i| (&i.id, i.hidden)).collect::<Vec<_>>());
            }
        }
    }
}
