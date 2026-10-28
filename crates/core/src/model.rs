//! Modelo de dados do simulador. Todo o conteúdo é FICTÍCIO e SINTÉTICO.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ModuleId {
    M501,
    M502,
    M503,
    M504,
    M505,
}

impl ModuleId {
    pub const ALL: [ModuleId; 5] = [ModuleId::M501, ModuleId::M502, ModuleId::M503, ModuleId::M504, ModuleId::M505];
    pub fn code(self) -> &'static str {
        match self {
            ModuleId::M501 => "5.01",
            ModuleId::M502 => "5.02",
            ModuleId::M503 => "5.03",
            ModuleId::M504 => "5.04",
            ModuleId::M505 => "5.05",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            ModuleId::M501 => "Exame pericial de mídias de armazenamento computacional",
            ModuleId::M502 => "Exame pericial de dispositivos computacionais móveis",
            ModuleId::M503 => "Exame pericial de local de informática",
            ModuleId::M504 => "Exame pericial de local de internet",
            ModuleId::M505 => "Apreensão de dispositivos computacionais móveis",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            ModuleId::M501 => "Mídias de armazenamento",
            ModuleId::M502 => "Dispositivos móveis",
            ModuleId::M503 => "Local de informática",
            ModuleId::M504 => "Local de internet",
            ModuleId::M505 => "Apreensão de móveis",
        }
    }
    pub fn from_code(s: &str) -> Option<ModuleId> {
        ModuleId::ALL.iter().copied().find(|m| m.code() == s)
    }
    /// O POP 5.05 indica “Estrutura básica do laudo: não se aplica” (p. 63).
    pub fn document(self) -> DocKind {
        match self {
            ModuleId::M505 => DocKind::TermoApreensao,
            _ => DocKind::Laudo,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocKind {
    Laudo,
    TermoApreensao,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Setting {
    Laboratorio,
    Local,
}

// ------------------------------------------------------------------ arquivos

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Texto,
    Documento,
    Planilha,
    Email,
    Imagem,
    Chat,
    HistoricoWeb,
    Compactado,
    ConteinerCifrado,
    Log,
    Config,
    Sistema,
    Audio,
    BancoDados,
}

impl FileType {
    pub fn label(self) -> &'static str {
        match self {
            FileType::Texto => "texto",
            FileType::Documento => "documento",
            FileType::Planilha => "planilha",
            FileType::Email => "e-mail",
            FileType::Imagem => "imagem",
            FileType::Chat => "conversa",
            FileType::HistoricoWeb => "histórico web",
            FileType::Compactado => "compactado",
            FileType::ConteinerCifrado => "contêiner cifrado",
            FileType::Log => "log",
            FileType::Config => "configuração",
            FileType::Sistema => "sistema",
            FileType::Audio => "áudio",
            FileType::BancoDados => "banco de dados",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recover {
    Integral,
    Parcial,
    Nenhuma,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VFile {
    pub id: String,
    pub path: String,
    pub size: u64,
    pub mtime: i64,
    pub ctime: i64,
    pub atime: i64,
    pub owner: String,
    pub ftype: FileType,
    pub deleted: bool,
    /// Para arquivos apagados: o que o modelo permite recuperar.
    pub recover: Option<Recover>,
    /// Arquivo em região de setores ilegíveis.
    pub bad_sector: bool,
    pub content: String,
    /// Texto obtido por OCR (imagem) ou transcrição (áudio) — exige processamento.
    pub ocr: Option<String>,
    pub meta: BTreeMap<String, String>,
    /// Senha do próprio arquivo (zip/documento cifrado).
    pub password: Option<String>,
    pub children: Vec<VFile>,
    /// Contêiner cifrado: partição interna.
    pub container: Option<Box<Partition>>,
    pub keyword: bool,
}

impl VFile {
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Partition {
    pub id: String,
    pub index: u32,
    pub fs: String,
    pub label: String,
    pub size: u64,
    pub files: Vec<VFile>,
    /// Arquivos de sistema sem interesse, sintetizados sob demanda a partir do índice.
    pub noise: u32,
    pub noise_roots: Vec<String>,
    /// Volume cifrado por software (BitLocker-like). `tpm`: depende do equipamento original.
    pub encrypted: Option<Encryption>,
    pub tz_offset: i32,
    pub dst_note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Encryption {
    pub scheme: String,
    pub password: String,
    pub tpm: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaKind {
    Hdd,
    Ssd,
    Nvme,
    Usb,
    Sd,
    FlashMovel,
}

impl MediaKind {
    pub fn label(self) -> &'static str {
        match self {
            MediaKind::Hdd => "Disco rígido (HDD)",
            MediaKind::Ssd => "SSD SATA",
            MediaKind::Nvme => "SSD NVMe (M.2)",
            MediaKind::Usb => "Pendrive USB",
            MediaKind::Sd => "Cartão microSD",
            MediaKind::FlashMovel => "Memória interna (dispositivo móvel)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Health {
    Ok,
    SetoresRuins,
    FalhaConector,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Media {
    pub id: String,
    pub kind: MediaKind,
    pub iface: String,
    pub brand: String,
    pub model: String,
    pub serial: String,
    pub capacity: u64,
    pub health: Health,
    pub bad_sectors: u32,
    /// Cifragem integral dependente do equipamento (SED/TPM): só decifra no equipamento original.
    pub fde: Option<Encryption>,
    pub partitions: Vec<Partition>,
    pub raid: Option<Raid>,
    /// Escritas que contaminaram a mídia original durante o caso.
    pub writes: Vec<String>,
    /// Mídia de trabalho produzida por espelhamento (clone) de outra.
    pub clone_of: Option<String>,
    pub sanitized: bool,
    pub is_work_media: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Raid {
    pub group: String,
    pub level: String,
    pub member: u32,
}

// ------------------------------------------------------------------ itens físicos

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Desktop,
    Notebook,
    Servidor,
    Celular,
    Tablet,
    Hdd,
    Ssd,
    Nvme,
    Pendrive,
    CartaoSd,
}

impl ItemKind {
    pub fn label(self) -> &'static str {
        match self {
            ItemKind::Desktop => "Computador (gabinete)",
            ItemKind::Notebook => "Notebook",
            ItemKind::Servidor => "Servidor",
            ItemKind::Celular => "Telefone celular",
            ItemKind::Tablet => "Tablet",
            ItemKind::Hdd => "Disco rígido",
            ItemKind::Ssd => "SSD SATA",
            ItemKind::Nvme => "SSD NVMe",
            ItemKind::Pendrive => "Pendrive",
            ItemKind::CartaoSd => "Cartão de memória",
        }
    }
    pub fn is_mobile(self) -> bool {
        matches!(self, ItemKind::Celular | ItemKind::Tablet)
    }
    pub fn is_computer(self) -> bool {
        matches!(self, ItemKind::Desktop | ItemKind::Notebook | ItemKind::Servidor)
    }
    pub fn is_media(self) -> bool {
        matches!(self, ItemKind::Hdd | ItemKind::Ssd | ItemKind::Nvme | ItemKind::Pendrive | ItemKind::CartaoSd)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Slot {
    pub id: String,
    pub label: String,
    pub iface: String,
    pub media_id: Option<String>,
    /// Componente que esconde o slot (POP 5.03 — mídias fisicamente ocultas).
    pub hidden_by: Option<String>,
    pub soldered: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub removed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimState {
    PinAtivo,
    SemPin,
    Puk,
    BloqueadoPermanente,
    Desbloqueado,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimWhere {
    Slot,
    Removido,
    AfixadoVerso,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimCard {
    pub iccid: String,
    pub operadora: String,
    pub pin: String,
    pub puk: String,
    pub pin_tries: u32,
    pub puk_tries: u32,
    pub state: SimState,
    pub place: SimWhere,
    pub slot: u32,
    pub contacts: Vec<(String, String)>,
    pub sms: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boot {
    Desligado,
    Bfu,
    Afu,
    Desbloqueado,
}

impl Boot {
    pub fn label(self) -> &'static str {
        match self {
            Boot::Desligado => "desligado",
            Boot::Bfu => "ligado, bloqueado (BFU — antes do primeiro desbloqueio)",
            Boot::Afu => "ligado, bloqueado (AFU — após o primeiro desbloqueio)",
            Boot::Desbloqueado => "ligado e desbloqueado",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rf {
    Nenhum,
    FaradayBag,
    Aluminio3,
    AluminioInsuficiente,
    SalaFaraday,
    Jammer,
}

impl Rf {
    pub fn isolates(self) -> bool {
        matches!(self, Rf::FaradayBag | Rf::Aluminio3 | Rf::SalaFaraday | Rf::Jammer)
    }
    pub fn label(self) -> &'static str {
        match self {
            Rf::Nenhum => "sem isolamento",
            Rf::FaradayBag => "Faraday bag",
            Rf::Aluminio3 => "papel-alumínio (3 camadas)",
            Rf::AluminioInsuficiente => "papel-alumínio (cobertura insuficiente)",
            Rf::SalaFaraday => "sala/caixa de Faraday",
            Rf::Jammer => "bloqueador de sinais ativo",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FsSupport {
    Sempre,
    Afu,
    Desbloqueado,
    Nao,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhoneState {
    pub os: String,
    pub os_version: String,
    pub imeis: Vec<String>,
    pub imei_label: Vec<String>,
    pub lock: String,
    pub passcode: String,
    pub smartlock: Option<String>,
    pub boot: Boot,
    pub battery: f64,
    pub charging: bool,
    pub battery_removable: bool,
    pub battery_removed: bool,
    pub airplane: bool,
    pub wifi: bool,
    pub bt: bool,
    pub cellular: bool,
    pub rf: Rf,
    pub external_battery: bool,
    pub sims: Vec<SimCard>,
    pub esim: Option<(String, String)>,
    pub sd_media: Option<String>,
    pub sd_inside: bool,
    pub encryption: String,
    pub support_logica: bool,
    pub support_fs: FsSupport,
    pub support_fisica: bool,
    pub alarms: Vec<String>,
    pub alarms_disabled: bool,
    /// Instante em que um comando remoto de restauração é enviado; executa se houver conectividade.
    pub remote_wipe_at: Option<i64>,
    pub wiped: bool,
    pub wipe_after_fails: Option<u32>,
    pub failed_unlocks: u32,
    pub second_space: Option<SecondSpace>,
    pub front_camera_covered: bool,
    pub sticker_on_screen: bool,
    pub screen_broken: bool,
    pub port_damaged: bool,
    pub data_media: String,
    pub model: String,
    pub users_known: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecondSpace {
    pub name: String,
    pub password: String,
    pub media_partition: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveState {
    pub logged_user: String,
    pub processes: Vec<(String, u32, String)>,
    /// Volume cifrado montado no momento (partição, letra/caminho).
    pub mounted_encrypted: Option<(String, String)>,
    pub ram_artifacts: Vec<(String, String, bool)>,
    pub screen: String,
    pub network_connected: bool,
    /// Acesso remoto ativo que pode apagar dados se não houver isolamento digital.
    pub remote_access: Option<(String, i64)>,
    pub memory_dump: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Physical {
    pub traces: Vec<String>,
    pub wet: bool,
    pub biohazard: bool,
    pub damage: Vec<String>,
    pub battery_swollen: bool,
    pub camouflaged: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Seal {
    pub number: String,
    pub intact: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub kind: ItemKind,
    pub label: String,
    pub brand: String,
    pub model: String,
    pub serial: String,
    pub description: String,
    pub seal: Option<Seal>,
    /// Lacre constante na documentação que acompanha o material.
    pub doc_seal: Option<String>,
    pub doc_description: Option<String>,
    pub power_on: bool,
    pub media: Vec<String>,
    pub slots: Vec<Slot>,
    pub components: Vec<Component>,
    pub phone: Option<PhoneState>,
    pub live: Option<LiveState>,
    pub physical: Physical,
    pub owner: Option<String>,
    pub hidden: bool,
    pub is_server: bool,
    /// Marcadores de procedimento físico (fotografado, embalado, etiquetado…).
    pub flags: BTreeMap<String, String>,
}

impl Item {
    pub fn flag(&self, k: &str) -> bool {
        self.flags.contains_key(k)
    }
    pub fn set(&mut self, k: &str, v: &str) {
        self.flags.insert(k.to_string(), v.to_string());
    }
}

// ------------------------------------------------------------------ rede emulada

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    pub rtype: String,
    pub value: String,
    pub ttl: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Registrant {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub country: String,
    pub privacy: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Domain {
    pub name: String,
    pub registrar: String,
    pub created: i64,
    pub expires: i64,
    pub registrant: Registrant,
    pub nameservers: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Host {
    pub ip: String,
    pub ptr: Option<String>,
    pub asn: String,
    pub org: String,
    pub geo: String,
    pub role: String,
    /// Servidor que registra visitantes e pode alertar o investigado (POP 5.04 §5).
    pub monitors: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Page {
    pub url: String,
    pub title: String,
    pub body: String,
    pub links: Vec<String>,
    pub unlisted: bool,
    pub redirect_to: Option<String>,
    pub removed_at: Option<i64>,
    /// Se o investigado for alertado, sai do ar após este atraso (segundos).
    pub removed_if_alerted_after: Option<i64>,
    pub keyword: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub url: String,
    pub at: i64,
    pub title: String,
    pub body: String,
    pub keyword: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CgnatRecord {
    pub public_ip: String,
    pub port_start: u32,
    pub port_end: u32,
    pub start: i64,
    pub end: i64,
    pub subscriber: String,
    pub private_ip: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Isp {
    pub name: String,
    pub asn: String,
    pub prefix: String,
    pub cgnat: bool,
    pub logs: Vec<CgnatRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cloud {
    pub provider: String,
    pub account: String,
    pub password: String,
    pub files: Vec<VFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetWorld {
    pub domains: Vec<Domain>,
    pub dns: Vec<DnsRecord>,
    pub hosts: Vec<Host>,
    pub pages: Vec<Page>,
    pub archive: Vec<Snapshot>,
    pub isps: Vec<Isp>,
    pub cloud: Option<Cloud>,
    /// IP público do perito conforme a conexão escolhida.
    pub expert_ips: BTreeMap<String, String>,
}

// ------------------------------------------------------------------ caso

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub role: String,
    pub email: String,
    pub phone: String,
    pub suspect: bool,
    pub birth_year: i64,
    pub pet: String,
    pub team: String,
    pub city: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SourceRef {
    File { media: String, file: String, path: String },
    Page { url: String },
    Archive { url: String },
    Ram { item: String },
    Cloud { path: String },
    Provided { file: String },
    /// Observação direta em campo (ex.: notificação na tela bloqueada, anotação ao lado do equipamento).
    Observed { item: String, what: String },
}

impl SourceRef {
    pub fn describe(&self) -> String {
        match self {
            SourceRef::File { media, path, .. } => format!("arquivo {} (mídia {})", path, media),
            SourceRef::Page { url } => format!("página {}", url),
            SourceRef::Archive { url } => format!("registro histórico de {}", url),
            SourceRef::Ram { item } => format!("memória volátil do item {}", item),
            SourceRef::Cloud { path } => format!("dados em nuvem {}", path),
            SourceRef::Provided { file } => format!("arquivo encaminhado {}", file),
            SourceRef::Observed { item, what } => format!("observação em campo ({}) — item {}", what, item),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Legal {
    pub mandado: bool,
    pub mandado_texto: String,
    pub autorizacao_nuvem: bool,
    pub autorizacao_dados: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledEvent {
    pub at: i64,
    pub kind: String,
    pub target: String,
    pub note: String,
    pub fired: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaseFile {
    pub id: String,
    pub seed: u32,
    pub gen_version: String,
    pub difficulty: u8,
    pub module: ModuleId,
    pub scenario: String,
    pub title: String,
    pub historico: String,
    pub requisitante: String,
    pub procedimento: String,
    pub objetivo: String,
    pub quesitos: Vec<String>,
    pub keyword: String,
    pub keyword_sources: Vec<SourceRef>,
    pub decoys: Vec<String>,
    pub start: i64,
    pub setting: Setting,
    pub legal: Legal,
    pub people: Vec<Person>,
    pub items: Vec<Item>,
    pub media: Vec<Media>,
    pub net: Option<NetWorld>,
    /// Material encaminhado em arquivo (ex.: cabeçalho .eml) para exame de internet.
    pub provided: Vec<VFile>,
    pub briefing: Vec<String>,
    pub events: Vec<ScheduledEvent>,
    /// Hash registrado em documento anterior (ex.: termo de apreensão), por mídia.
    pub documented_hashes: BTreeMap<String, String>,
    /// Documentos exigidos ao final do caso.
    pub required_docs: Vec<DocKind>,
    /// Senhas fornecidas no termo/ofício que acompanha o material.
    pub provided_passwords: Vec<(String, String)>,
    /// Operadoras fictícias: PIN padrão divulgado.
    pub operator_default_pins: Vec<(String, String)>,
    /// Vestígios não digitais da cena (sangue, cabelo, bituca…) — gerador 1.2.0+.
    #[serde(default)]
    pub vestigios: Vec<Vestigio>,
}

/// Vestígio físico na cena ou no material. O perito de informática sinaliza, fotografa e
/// comunica; a coleta é da equipe especializada.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Vestigio {
    pub id: String,
    /// sangue | cabelo | bituca | copo | pegada
    pub kind: String,
    pub label: String,
    pub desc: String,
    /// Item onde está (None = no ambiente).
    pub on_item: Option<String>,
    pub biologico: bool,
    /// Âncora de posição para a cena 3D: "item", "piso_mesa", "teclado", "mesa", "bancada", "cinzeiro".
    pub spot: String,
}

impl CaseFile {
    pub fn suspect(&self) -> &Person {
        self.people.iter().find(|p| p.suspect).unwrap_or(&self.people[0])
    }
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id.eq_ignore_ascii_case(id))
    }
    pub fn media_by_id(&self, id: &str) -> Option<&Media> {
        self.media.iter().find(|m| m.id.eq_ignore_ascii_case(id))
    }
}
