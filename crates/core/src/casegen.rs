//! Gerador procedural de casos. A mesma (semente, módulo, dificuldade, GEN_VERSION)
//! reproduz exatamente o mesmo caso. Os vestígios partem de uma "história" única por
//! caso, de modo que arquivos, horários, rede e pessoas sejam coerentes entre si.
//!
//! Todo o conteúdo é FICTÍCIO. Os cenários de crime servem apenas de moldura narrativa
//! para o treinamento pericial; descrevem a investigação, não instruções para cometê-los.

use crate::content::*;
use crate::model::*;
use crate::rng::Rng;
use crate::util::{fmt_dt, ts, BRT};
use std::collections::BTreeMap;

pub const GEN_VERSION: &str = "1.2.0";
/// Versões do gerador que esta compilação reproduz com fidelidade (salvamentos antigos).
pub const SUPPORTED_GEN_VERSIONS: &[&str] = &["1.0.0", "1.1.0", "1.2.0"];
/// Rótulo da semente do conteúdo-base. Fica fixo entre versões para que um mesmo ID de
/// caso mantenha o mesmo conteúdo-base; versões novas só ACRESCENTAM extensões, com
/// sementes próprias (ver `extensions`).
const BASE_LABEL: &str = "1.0.0";

pub fn case_id(module: ModuleId, difficulty: u8, seed: u32) -> String {
    format!("C{}-D{}-{:08X}", module.code().replace('.', ""), difficulty, seed)
}

/// Interpreta um identificador de caso, para reproduzir um caso a partir do ID.
pub fn parse_case_id(id: &str) -> Option<(ModuleId, u8, u32)> {
    let id = id.trim().to_uppercase();
    let rest = id.strip_prefix('C')?;
    let mut parts = rest.split('-');
    let m = parts.next()?;
    let d = parts.next()?.strip_prefix('D')?.parse::<u8>().ok()?;
    let seed = u32::from_str_radix(parts.next()?, 16).ok()?;
    if m.len() < 2 {
        return None;
    }
    let module = ModuleId::from_code(&format!("{}.{}", &m[..1], &m[1..]))?;
    if !(1..=4).contains(&d) {
        return None;
    }
    Some((module, d, seed))
}

pub struct Scenario {
    pub key: &'static str,
    pub title: &'static str,
    pub prefix: &'static str,
    pub crime: &'static str,
    /// Descrição neutra do que se investiga (moldura narrativa).
    pub summary: &'static str,
}

pub const SCENARIOS: &[Scenario] = &[
    Scenario {
        key: "vazamento",
        title: "Suspeita de vazamento de projeto industrial sigiloso",
        prefix: "PROJETO",
        crime: "violação de segredo de empresa (fictício)",
        summary: "A empresa suspeita que um documento técnico confidencial saiu do grupo de trabalho autorizado.",
    },
    Scenario {
        key: "ameaca",
        title: "Mensagens de ameaça com exigência de pagamento",
        prefix: "OPERAÇÃO",
        crime: "extorsão (fictício)",
        summary: "A vítima recebeu mensagens exigindo pagamento sob ameaça de divulgar dados pessoais.",
    },
    Scenario {
        key: "pagina-falsa",
        title: "Página falsa imitando instituição (fraude eletrônica)",
        prefix: "LOTE",
        crime: "estelionato eletrônico (fictício)",
        summary: "Vítimas relataram ter informado dados em uma página que imitava um serviço legítimo.",
    },
    Scenario {
        key: "falso-investimento",
        title: "Oferta fraudulenta de investimento por aplicativo de mensagens",
        prefix: "PLANO",
        crime: "estelionato (fictício)",
        summary: "Investidores relatam uma oferta de rendimento que deixou de pagar os resgates prometidos.",
    },
];

fn scenario_for(module: ModuleId, rng: &mut Rng) -> &'static Scenario {
    let keys: &[&str] = match module {
        ModuleId::M501 => &["vazamento", "ameaca"],
        ModuleId::M502 => &["falso-investimento", "vazamento"],
        ModuleId::M503 => &["vazamento", "ameaca"],
        ModuleId::M504 => &["pagina-falsa", "ameaca"],
        ModuleId::M505 => &["falso-investimento", "pagina-falsa"],
    };
    let k = *rng.pick(keys);
    SCENARIOS.iter().find(|s| s.key == k).unwrap()
}

pub struct Ctx {
    pub rng: Rng,
    pub d: u8,
    pub start: i64,
    pub keyword: String,
    pub decoy: String,
    pub suspect: Person,
    pub victim: Person,
    pub other: Person,
    pub company: (String, String),
    pub scenario: &'static Scenario,
    pub fid: u32,
}

impl Ctx {
    pub fn fid(&mut self, prefix: &str) -> String {
        self.fid += 1;
        format!("{}-{:04}", prefix, self.fid)
    }
    pub fn file(&mut self, path: &str, ftype: FileType, content: String, mtime: i64, owner: &str) -> VFile {
        let id = self.fid("F");
        let mult = match ftype {
            FileType::Documento | FileType::Planilha => 9,
            FileType::Imagem => 900,
            FileType::Audio => 1500,
            FileType::Compactado | FileType::ConteinerCifrado => 40,
            _ => 1,
        };
        VFile {
            id,
            path: path.to_string(),
            size: (content.len() as u64).max(64) * mult,
            mtime,
            ctime: mtime - self.rng.int(0, 3) * 86400,
            atime: mtime + self.rng.int(0, 5) * 3600,
            owner: owner.to_string(),
            ftype,
            deleted: false,
            recover: None,
            bad_sector: false,
            content,
            ocr: None,
            meta: BTreeMap::new(),
            password: None,
            children: vec![],
            container: None,
            keyword: false,
        }
    }
    pub fn deleted(&mut self, mut f: VFile, r: Recover) -> VFile {
        f.deleted = true;
        f.recover = Some(r);
        f
    }
    pub fn t(&mut self, days_before_min: i64, days_before_max: i64) -> i64 {
        self.start - self.rng.int(days_before_min * 86400, days_before_max * 86400)
    }
    /// Documento-chave do caso: contém a expressão investigada, em tom narrativo e genérico.
    pub fn keyword_doc(&mut self) -> String {
        let k = self.keyword.clone();
        let comp = self.company.0.clone();
        let s = self.suspect.name.clone();
        match self.scenario.key {
            "vazamento" => format!(
                "CONFIDENCIAL — {}\nReferência interna: {}\n\nDocumento técnico de circulação restrita ao grupo de trabalho.\nEsta cópia foi preparada por {} e não deveria ter saído do grupo.",
                comp, k, s
            ),
            "ameaca" => format!(
                "Anotações — {}\nReferência: {}\n\nRegistro de contatos feitos com a vítima {} e de prazos mencionados nas mensagens.\nMaterial reunido para a investigação.",
                comp, k, self.victim.name
            ),
            "pagina-falsa" => format!(
                "Anotações internas — {}\nReferência: {}\n\nControle de uma página que imitava um serviço legítimo.\nRegistro associado a {}.",
                comp, k, s
            ),
            _ => format!(
                "Anotações — {}\nReferência: {}\n\nRegistro da oferta de investimento questionada e da lista de pessoas contatadas.\nCoordenação atribuída a {}.",
                comp, k, s
            ),
        }
    }
}

fn make_keyword(rng: &mut Rng, scen: &Scenario) -> (String, String) {
    let w = *rng.pick(CODEWORDS);
    let n = rng.int(11, 97);
    let mut dn = n + if rng.chance(0.5) { 1 } else { -1 };
    if dn == n {
        dn += 2;
    }
    (format!("{} {}-{}", scen.prefix, w, n), format!("{} {}-{}", scen.prefix, w, dn))
}

pub fn expert_ips() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("institucional".into(), "198.51.100.10".into());
    m.insert("dedicada".into(), "203.0.113.200".into());
    m.insert("anonima".into(), "192.0.2.250".into());
    m
}

pub fn new_media(c: &mut Ctx, id: &str, kind: MediaKind, cap_gb: u64) -> Media {
    let brand = c.rng.pick(DISK_BRANDS).to_string();
    let iface = match kind {
        MediaKind::Hdd | MediaKind::Ssd => "SATA",
        MediaKind::Nvme => "NVMe",
        MediaKind::Usb => "USB",
        MediaKind::Sd => "microSD",
        MediaKind::FlashMovel => "interna",
    };
    Media {
        id: id.to_string(),
        kind,
        iface: iface.to_string(),
        model: format!("{}{}-{}", brand.chars().take(2).collect::<String>().to_uppercase(), cap_gb, c.rng.hex(3)),
        brand,
        serial: format!("{}{}", c.rng.hex(4), c.rng.digits(6)),
        capacity: cap_gb * 1_000_000_000,
        health: Health::Ok,
        bad_sectors: 0,
        fde: None,
        partitions: vec![],
        raid: None,
        writes: vec![],
        clone_of: None,
        sanitized: false,
        is_work_media: false,
    }
}

pub fn partition(id: &str, index: u32, fs: &str, label: &str, size: u64, noise: u32, roots: &[&str]) -> Partition {
    Partition {
        id: id.to_string(),
        index,
        fs: fs.to_string(),
        label: label.to_string(),
        size,
        files: vec![],
        noise,
        noise_roots: roots.iter().map(|s| s.to_string()).collect(),
        encrypted: None,
        tz_offset: BRT,
        dst_note: None,
    }
}

pub fn base_item(c: &mut Ctx, id: &str, kind: ItemKind, label: &str) -> Item {
    let brand = if kind.is_computer() { c.rng.pick(PC_BRANDS).to_string() } else { c.rng.pick(DISK_BRANDS).to_string() };
    Item {
        id: id.to_string(),
        kind,
        label: label.to_string(),
        model: format!("{}-{}", c.rng.hex(2), c.rng.int(100, 990)),
        brand,
        serial: format!("SN{}{}", c.rng.hex(3), c.rng.digits(7)),
        description: String::new(),
        seal: None,
        doc_seal: None,
        doc_description: None,
        power_on: false,
        media: vec![],
        slots: vec![],
        components: vec![],
        phone: None,
        live: None,
        physical: Physical::default(),
        owner: None,
        hidden: false,
        is_server: false,
        flags: BTreeMap::new(),
    }
}

pub fn seal_number(rng: &mut Rng) -> String {
    format!("{}{}", rng.int(1, 9), rng.digits(6))
}

/// Selo com eventual divergência documental (dificuldade ≥ 2) — deve ser registrada
/// pelo perito antes do deslacre (POP 5.01 §4.1; POP 5.03 §4.5).
pub fn seal_item(c: &mut Ctx, it: &mut Item, mismatch_p: f64) {
    let n = seal_number(&mut c.rng);
    let doc = if c.d >= 2 && c.rng.chance(mismatch_p) {
        let mut ch: Vec<char> = n.chars().collect();
        ch.swap(2, 3);
        if ch[2] == ch[3] {
            ch[3] = if ch[3] == '9' { '0' } else { ((ch[3] as u8) + 1) as char };
        }
        ch.into_iter().collect()
    } else {
        n.clone()
    };
    it.seal = Some(Seal { number: n, intact: true });
    it.doc_seal = Some(doc);
}

fn people_for(rng: &mut Rng, company_domain: &str) -> (Person, Person, Person) {
    let mut s = person(rng, "investigado(a)", "correio.sim", true);
    let v = person(rng, "vítima / representante da empresa", &format!("{}.sim", company_domain), false);
    let mut o = person(rng, "colega de trabalho", &format!("{}.sim", company_domain), false);
    if o.name == s.name {
        o.name = format!("{} Filho", o.name);
    }
    if v.name == s.name {
        s.name = format!("{} Júnior", s.name);
    }
    (s, v, o)
}

pub fn generate(module: ModuleId, difficulty: u8, seed: u32) -> CaseFile {
    generate_v(module, difficulty, seed, GEN_VERSION)
}

/// Caso com as regras clássicas (gerador 1.1.0: ações sem exigência de ferramenta na mão).
/// Usado pelos testes de fluxo e de compatibilidade de salvamentos antigos.
pub fn generate_classic(module: ModuleId, difficulty: u8, seed: u32) -> CaseFile {
    generate_v(module, difficulty, seed, "1.1.0")
}

/// Gera o caso como a versão `version` do gerador o faria (deve estar em
/// `SUPPORTED_GEN_VERSIONS`; versões desconhecidas caem na atual).
pub fn generate_v(module: ModuleId, difficulty: u8, seed: u32, version: &str) -> CaseFile {
    let version = if SUPPORTED_GEN_VERSIONS.contains(&version) { version } else { GEN_VERSION };
    let mut case = generate_base(module, difficulty, seed);
    case.gen_version = version.to_string();
    crate::casegen_ext::apply(&mut case, version);
    case
}

fn generate_base(module: ModuleId, difficulty: u8, seed: u32) -> CaseFile {
    let d = difficulty.clamp(1, 4);
    let mut rng = Rng::for_label(seed, &format!("{}|{}|{}", BASE_LABEL, module.code(), d));
    let scenario = scenario_for(module, &mut rng);
    let company = rng.pick(COMPANIES);
    let (suspect, victim, other) = people_for(&mut rng, company.1);
    let (keyword, decoy) = make_keyword(&mut rng, scenario);
    let day = rng.int(0, 200);
    let start = ts(2026, 2, 2, 8, 30, BRT) + day * 86400 + rng.int(0, 90) * 60;
    let mut c = Ctx {
        rng,
        d,
        start,
        keyword,
        decoy,
        suspect,
        victim,
        other,
        company: (company.0.to_string(), company.1.to_string()),
        scenario,
        fid: 0,
    };
    let mut case = CaseFile {
        id: case_id(module, d, seed),
        seed,
        gen_version: GEN_VERSION.to_string(),
        difficulty: d,
        module,
        scenario: scenario.key.to_string(),
        title: scenario.title.to_string(),
        historico: String::new(),
        requisitante: format!("Delegacia de Repressão a Crimes Cibernéticos de {}", c.suspect.city),
        procedimento: format!("Inquérito Policial nº {}/{}", c.rng.int(100, 2999), 2026),
        objetivo: String::new(),
        quesitos: vec![],
        keyword: c.keyword.clone(),
        keyword_sources: vec![],
        decoys: vec![],
        start,
        setting: Setting::Laboratorio,
        legal: Legal { mandado: true, mandado_texto: String::new(), autorizacao_nuvem: false, autorizacao_dados: true },
        people: vec![],
        items: vec![],
        media: vec![],
        net: None,
        provided: vec![],
        briefing: vec![],
        events: vec![],
        documented_hashes: BTreeMap::new(),
        required_docs: vec![module.document()],
        provided_passwords: vec![],
        operator_default_pins: OPERATORS.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        vestigios: vec![],
    };
    match module {
        ModuleId::M501 => crate::casegen_disk::gen_501(&mut c, &mut case),
        ModuleId::M502 => crate::casegen_mobile::gen_502(&mut c, &mut case),
        ModuleId::M503 => crate::casegen_scene::gen_503(&mut c, &mut case),
        ModuleId::M504 => crate::casegen_net::gen_504(&mut c, &mut case),
        ModuleId::M505 => crate::casegen_mobile::gen_505(&mut c, &mut case),
    }
    case.people = vec![c.suspect.clone(), c.victim.clone(), c.other.clone()];
    case
}

/// Material plausível de uma pasta de usuário Windows (usado por 5.01 e 5.03).
pub fn windows_user_files(c: &mut Ctx, user: &str, suspect_name: &str) -> Vec<VFile> {
    let home = format!("/Users/{}", user);
    let mut v = vec![];
    let owner = user.to_string();
    for (name, topic) in [("Relatorio_mensal.docx", "relatório mensal"), ("Orcamento_2026.xlsx", "orçamento anual"), ("Ata_reuniao.docx", "ata de reunião")] {
        let t = c.t(3, 60);
        let p = filler_paragraph(&mut c.rng, topic);
        let ft = if name.ends_with("xlsx") { FileType::Planilha } else { FileType::Documento };
        v.push(c.file(&format!("{}/Documents/{}", home, name), ft, document(topic, suspect_name, t, &p), t, &owner));
    }
    let t = c.t(1, 20);
    let hist = browser_history(&mut c.rng, c.start, &[]);
    v.push(c.file(&format!("{}/AppData/Local/Navegador/History.csv", home), FileType::HistoricoWeb, hist, t, &owner));
    let t = c.t(1, 10);
    let log = system_log(&mut c.rng, c.start, user, &[]);
    v.push(c.file("/Windows/Logs/eventos.log", FileType::Log, log, t, "SYSTEM"));
    let t = c.t(30, 300);
    let pet = c.suspect.pet.clone();
    let by = c.suspect.birth_year;
    let fam = suspect_name.split(' ').last().unwrap_or("").to_string();
    let dg = c.rng.digits(4);
    let mut img = c.file(&format!("{}/Pictures/IMG_{}.jpg", home, dg), FileType::Imagem, "[imagem JPEG — use OCR para extrair texto]".into(), t, &owner);
    img.ocr = Some(format!("Festa do {}! Nascido em {} — família {}", pet, by, fam));
    img.meta.insert("Câmera".into(), "Kairo M22".into());
    img.meta.insert("DataOriginal".into(), fmt_dt(t, BRT));
    v.push(img);
    v
}

pub fn case_cloud_password(files: &[VFile]) -> String {
    files
        .iter()
        .find(|f| f.path.ends_with("senhas_exportadas.csv"))
        .and_then(|f| f.content.lines().nth(1).map(|l| l.split(';').nth(2).unwrap_or("").to_string()))
        .unwrap_or_default()
}
