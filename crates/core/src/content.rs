//! Vocabulário e geradores de conteúdo sintético (pt-BR). Nomes, empresas, domínios e
//! endereços são fictícios; IPs usam faixas reservadas para documentação (RFC 5737).

use crate::model::Person;
use crate::rng::Rng;
use crate::util::{fmt_dt, fmt_rfc5322, BRT};

pub const FIRST: &[&str] = &[
    "Ana", "Bruno", "Carla", "Diego", "Elisa", "Fábio", "Gabriela", "Heitor", "Isadora", "João", "Karina", "Leandro",
    "Marina", "Nelson", "Olívia", "Paulo", "Quitéria", "Rafael", "Sílvia", "Tiago", "Úrsula", "Vinícius", "Wanda", "Yago",
    "Renata", "Otávio", "Luíza", "Caio", "Débora", "Murilo",
];
pub const LAST: &[&str] = &[
    "Albuquerque", "Barreto", "Cavalcanti", "Dorneles", "Esteves", "Figueiró", "Guimarães", "Holanda", "Ibiapina",
    "Junqueira", "Lacerda", "Meirelles", "Nogueira", "Orsini", "Pimentel", "Quaresma", "Rezende", "Salvatierra",
    "Teixeira", "Uchoa", "Valadares", "Xavier", "Zanetti", "Brandão", "Coutinho",
];
pub const PETS: &[&str] = &["Thor", "Mel", "Pipoca", "Bidu", "Luna", "Fred", "Nina", "Paçoca", "Zeca", "Amora", "Tufão", "Biscoito"];
pub const TEAMS: &[&str] = &["Ribeirão", "Serrano", "Atlântico", "Cruzeirinho", "Ipiranga", "Vale Verde", "Marítimo"];
pub const CITIES: &[&str] = &["Campinas", "Uberlândia", "Joinville", "Londrina", "Feira de Santana", "Juiz de Fora", "Caruaru", "Petrolina", "Maringá"];
pub const COMPANIES: &[(&str, &str)] = &[
    ("Altavia Componentes Ltda.", "altavia"),
    ("Metalúrgica Pedra Azul S.A.", "pedraazul"),
    ("Grupo Solaris Engenharia", "solarisengenharia"),
    ("Cooperativa Vale do Ipê", "valedoipe"),
    ("Laboratórios Nébula Ltda.", "nebulalab"),
];
pub const CODEWORDS: &[&str] = &[
    "ÍCARO", "CORVO", "ANDORINHA", "SAFIRA", "TAMANDUÁ", "BÚSSOLA", "GIRASSOL", "ÂNCORA", "JACARANDÁ", "PELICANO",
    "OBSIDIANA", "CARCARÁ", "MANDACARU", "AROEIRA",
];
pub const OPERATORS: &[(&str, &str)] = &[("Vivacel", "8486"), ("Teleonda", "1234"), ("Claronet", "3636"), ("Sinalmax", "0000")];
pub const MOBILE_MODELS: &[(&str, &str, &str)] = &[
    // (marca, modelo, SO) — fabricantes fictícios
    ("Lumina", "X5 Pro", "Android"),
    ("Nokto", "A3s", "Android"),
    ("Pomar", "Fone 13", "iOS"),
    ("Kairo", "M22", "Android"),
    ("Pomar", "Fone 11", "iOS"),
];
pub const DISK_BRANDS: &[&str] = &["Seagal", "Westerly", "Toshira", "Kingsvale", "Crucible", "Samsang"];
pub const PC_BRANDS: &[&str] = &["Positivo-X", "Lenovix", "Dellar", "Acerbo", "HPX"];

pub fn person(rng: &mut Rng, role: &str, domain: &str, suspect: bool) -> Person {
    let first = *rng.pick(FIRST);
    let last = *rng.pick(LAST);
    let user = crate::util::fold(&format!("{}.{}", first, last));
    Person {
        name: format!("{} {}", first, last),
        role: role.to_string(),
        email: format!("{}@{}", user, domain),
        phone: format!("+55 {} 9{}-{}", rng.int(11, 99), rng.digits(4), rng.digits(4)),
        suspect,
        birth_year: rng.int(1972, 2001),
        pet: rng.pick(PETS).to_string(),
        team: rng.pick(TEAMS).to_string(),
        city: rng.pick(CITIES).to_string(),
    }
}

pub fn first_name(p: &Person) -> &str {
    p.name.split(' ').next().unwrap_or(&p.name)
}

pub fn login(p: &Person) -> String {
    crate::util::fold(first_name(p))
}

/// Senha derivada de dados biográficos — o tipo de padrão que o POP 5.01 §4.5 sugere
/// explorar na geração de dicionários (dados biográficos + regras de alteração).
pub fn bio_password(rng: &mut Rng, p: &Person) -> String {
    let yy = p.birth_year % 100;
    match rng.int(0, 4) {
        0 => format!("{}{}", p.pet, p.birth_year),
        1 => format!("{}{:02}!", p.pet.to_lowercase(), yy),
        2 => format!("{}@{}", p.team.replace(' ', ""), p.birth_year),
        3 => format!("{}{}", leet(&p.pet), yy),
        _ => format!("{}{}", p.pet, p.birth_year),
    }
}

pub fn leet(s: &str) -> String {
    s.chars().map(|c| match c { 'a' => '@', 'e' => '3', 'o' => '0', 'i' => '1', c => c }).collect()
}

pub fn ascii_word(s: &str) -> String {
    crate::util::fold(s).chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// Mensagem de e-mail completa (RFC 5322) com trilha de campos Received.
pub struct MailSpec<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub subject: &'a str,
    pub body: &'a str,
    pub date: i64,
    pub offset: i32,
    pub received: Vec<String>,
    pub extra: Vec<(String, String)>,
    pub message_id: String,
}

pub fn render_mail(m: &MailSpec) -> String {
    let mut s = String::new();
    for r in &m.received {
        s.push_str("Received: ");
        s.push_str(r);
        s.push('\n');
    }
    for (k, v) in &m.extra {
        s.push_str(&format!("{}: {}\n", k, v));
    }
    s.push_str(&format!("Message-ID: <{}>\n", m.message_id));
    s.push_str(&format!("Date: {}\n", fmt_rfc5322(m.date, m.offset)));
    s.push_str(&format!("From: {}\nTo: {}\nSubject: {}\nMIME-Version: 1.0\nContent-Type: text/plain; charset=UTF-8\n\n", m.from, m.to, m.subject));
    s.push_str(m.body);
    s.push('\n');
    s
}

pub fn chat_line(t: i64, who: &str, text: &str) -> String {
    format!("[{}] {}: {}", fmt_dt(t, BRT), who, text)
}

pub fn filler_paragraph(rng: &mut Rng, topic: &str) -> String {
    let a = [
        "Conforme alinhado na reunião de segunda-feira,",
        "Segue, para conhecimento,",
        "Registramos abaixo",
        "Em complemento ao relatório anterior,",
        "Para fins de controle interno,",
    ];
    let b = [
        "o cronograma permanece sujeito a revisão pela diretoria.",
        "os valores apresentados são estimativas preliminares.",
        "as pendências deverão ser tratadas até o fechamento do mês.",
        "não há alterações relevantes em relação à versão anterior.",
        "o fornecedor confirmou o prazo de entrega informado.",
    ];
    format!("{} {} — {}", rng.pick(&a), topic, rng.pick(&b))
}

pub fn browser_history(rng: &mut Rng, start: i64, extra: &[(String, String)]) -> String {
    let base = [
        ("https://noticias.exemplo.sim/economia", "Economia — Notícias"),
        ("https://busca.exemplo.sim/?q=previs%C3%A3o+do+tempo", "previsão do tempo — Busca"),
        ("https://webmail.correio.sim/inbox", "Caixa de entrada"),
        ("https://video.exemplo.sim/watch?v=a81kq", "Receita de pão caseiro"),
        ("https://loja.exemplo.sim/carrinho", "Carrinho de compras"),
        ("https://mapas.exemplo.sim/rota", "Rotas"),
    ];
    let mut rows = vec!["visit_time_utc;url;title".to_string()];
    let mut t = start - rng.int(5, 20) * 86400;
    for _ in 0..rng.int(8, 14) {
        let (u, ti) = rng.pick(&base);
        t += rng.int(600, 30000);
        rows.push(format!("{};{};{}", crate::util::fmt_iso(t), u, ti));
    }
    for (u, ti) in extra {
        t += rng.int(600, 9000);
        rows.push(format!("{};{};{}", crate::util::fmt_iso(t), u, ti));
    }
    rows.join("\n")
}

pub fn system_log(rng: &mut Rng, start: i64, user: &str, extra: &[String]) -> String {
    let ev = [
        "Serviço de atualização iniciado",
        "Logon interativo bem-sucedido",
        "Dispositivo USB conectado (classe: armazenamento em massa)",
        "Dispositivo USB removido",
        "Desligamento solicitado pelo usuário",
        "Sincronização de horário concluída",
    ];
    let mut t = start - 7 * 86400;
    let mut out = vec![];
    for _ in 0..rng.int(10, 16) {
        t += rng.int(1800, 40000);
        out.push(format!("{} host={} user={} evento=\"{}\"", crate::util::fmt_iso(t), "WS-01", user, rng.pick(&ev)));
    }
    for e in extra {
        out.push(e.clone());
    }
    out.join("\n")
}

/// Bloco de “documento” plausível com título, data e corpo.
pub fn document(title: &str, author: &str, date: i64, body: &str) -> String {
    format!("{}\nAutor: {}\nData: {}\n\n{}\n", title, author, crate::util::fmt_date(date, BRT), body)
}
