use pericia_core::casegen::{case_id, generate};
use pericia_core::model::*;
fn main() {
    let fde = (1..200).find(|&s| generate(ModuleId::M501, 3, s).media.iter().any(|m| m.fde.is_some())).unwrap();
    let c = generate(ModuleId::M501, 3, fde);
    println!("fde {} senha {}", case_id(ModuleId::M501, 3, fde), c.media[0].fde.as_ref().unwrap().password);
    for auth in [true, false] {
        let s = (1..200).find(|&s| { let c = generate(ModuleId::M501, 4, s); c.net.as_ref().and_then(|n| n.cloud.as_ref()).is_some() && c.legal.autorizacao_nuvem == auth }).unwrap();
        let c = generate(ModuleId::M501, 4, s);
        let cl = c.net.as_ref().unwrap().cloud.as_ref().unwrap();
        println!("nuvem auth={} {} conta {} senha {}", auth, case_id(ModuleId::M501, 4, s), cl.account, cl.password);
    }
}
