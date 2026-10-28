use pericia_core::casegen::generate;
use pericia_core::engine::Sim;
use pericia_core::model::*;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (m, d, sd) = pericia_core::casegen::parse_case_id(&args[1]).unwrap();
    let mut s = Sim::new(generate(m, d, sd));
    for v in &s.case.vestigios { println!("{} {} {:?} {}", v.id, v.label, v.on_item, v.spot); }
    for it in &s.case.items { println!("{} {:?} slots {:?} media {:?} bio {}", it.id, it.kind, it.slots.iter().map(|x| (&x.id, &x.iface, &x.media_id)).collect::<Vec<_>>(), it.media, it.physical.biohazard); }
    for l in args[2..].iter() { let o = s.exec(l); println!("> {}\n{}{}", l, if o.ok {""} else {"ERRO: "}, o.text); }
    println!("--- faltas: {:?}", s.faults);
    for (ok, l) in pericia_core::assess::checklist(&s) { println!("[{}] {}", if ok {"x"} else {" "}, l); }
}
