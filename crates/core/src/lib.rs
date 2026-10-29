//! pericia_core — motor do simulador de informática forense.
//!
//! Não depende de nenhuma biblioteca gráfica nem de rede: todo o conteúdo é interno,
//! o que garante o isolamento exigido pelos POPs (a "internet" é inteiramente emulada).

pub mod rng;
pub mod sha256;
pub mod util;
pub mod model;
pub mod content;
pub mod casegen;
pub mod casegen_disk;
pub mod casegen_mobile;
pub mod casegen_scene;
pub mod casegen_net;
pub mod vfs;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
pub mod audit;
pub mod engine;
pub mod tools;
pub mod report;
pub mod assess;
pub mod pdf;
pub mod manual;
pub mod osapi;
pub mod os;
