//! Gerador de casos do POP 5.04 — exame de local de internet.
//! Constrói um "mundo de rede" coerente: domínios, DNS, hosts, páginas, e-mail,
//! logs de ISP (inclusive CGNAT) e registros históricos, tudo interligado.

use crate::casegen::*;
use crate::content::*;
use crate::model::*;
use crate::util::{fmt_rfc5322, BRT};

/// Monta um IP em faixa de documentação (RFC 5737) de forma determinística.
fn doc_ip(c: &mut Ctx, block: u8) -> String {
    let base = match block % 3 {
        0 => "192.0.2",
        1 => "198.51.100",
        _ => "203.0.113",
    };
    format!("{}.{}", base, c.rng.int(2, 254))
}

pub fn gen_504(c: &mut Ctx, case: &mut CaseFile) {
    case.setting = Setting::Laboratorio;
    let suspect_name = c.suspect.name.clone();
    let k = c.keyword.clone();

    // --- domínio-alvo (página fraudulenta ou de divulgação)
    let (brand, slug) = *c.rng.pick(COMPANIES);
    let bad_slug = format!("{}-{}", slug, c.rng.pick(&["seguro", "acesso", "cliente", "portal"]));
    let domain_name = format!("{}.sim", bad_slug);
    let web_ip = doc_ip(c, 2);
    let mail_ip = doc_ip(c, 1);
    let ns_ip = doc_ip(c, 0);
    let created = c.start - c.rng.int(10, 60) * 86400;

    // registrante: dados possivelmente FALSOS (o POP 5.04 alerta para isso)
    let fake_name = format!("{} {}", c.rng.pick(FIRST), c.rng.pick(LAST));
    let privacy = c.d >= 3;
    let domain = Domain {
        name: domain_name.clone(),
        registrar: "RegistroFicticio.sim".into(),
        created,
        expires: created + 365 * 86400,
        registrant: Registrant {
            name: if privacy { "[protegido por serviço de privacidade]".into() } else { fake_name.clone() },
            email: if privacy { "contato@privacidade.sim".into() } else { format!("{}@correio.sim", ascii_word(&fake_name)) },
            phone: format!("+55 {} 9{}-{}", c.rng.int(11, 99), c.rng.digits(4), c.rng.digits(4)),
            country: "BR".into(),
            privacy,
        },
        nameservers: vec![format!("ns1.{}", domain_name), format!("ns2.{}", domain_name)],
    };

    let dns = vec![
        DnsRecord { name: domain_name.clone(), rtype: "A".into(), value: web_ip.clone(), ttl: 300 },
        DnsRecord { name: format!("www.{}", domain_name), rtype: "CNAME".into(), value: domain_name.clone(), ttl: 300 },
        DnsRecord { name: format!("mail.{}", domain_name), rtype: "A".into(), value: mail_ip.clone(), ttl: 300 },
        DnsRecord { name: domain_name.clone(), rtype: "MX".into(), value: format!("10 mail.{}", domain_name), ttl: 300 },
        DnsRecord { name: domain_name.clone(), rtype: "NS".into(), value: format!("ns1.{}", domain_name), ttl: 86400 },
        DnsRecord { name: format!("ns1.{}", domain_name), rtype: "A".into(), value: ns_ip.clone(), ttl: 86400 },
        DnsRecord { name: web_ip.clone(), rtype: "PTR".into(), value: format!("host-{}.hostprov.sim", web_ip.replace('.', "-")), ttl: 3600 },
    ];

    let hosts = vec![
        Host { ip: web_ip.clone(), ptr: Some(format!("host-{}.hostprov.sim", web_ip.replace('.', "-"))), asn: "AS64500".into(), org: "HostProv Hospedagem Ltda (fictícia)".into(), geo: "São Paulo, BR".into(), role: "web".into(), monitors: c.d >= 3 },
        Host { ip: mail_ip.clone(), ptr: Some(format!("mail-{}.hostprov.sim", mail_ip.replace('.', "-"))), asn: "AS64500".into(), org: "HostProv Hospedagem Ltda (fictícia)".into(), geo: "São Paulo, BR".into(), role: "mail".into(), monitors: false },
    ];

    // --- páginas navegáveis
    let home_body = format!(
        "{}\n\nEsta página imita a identidade visual de {} para induzir o visitante a informar seus dados.\n[Formulário de acesso — apenas ilustrativo no simulador]\n\nReferência interna do caso: {}.",
        c.scenario.summary, brand, k
    );
    let home = Page {
        url: format!("https://{}/", domain_name),
        title: format!("Acesso — {} (página falsa)", brand),
        body: home_body,
        links: vec![format!("https://{}/termos", domain_name), format!("https://{}/painel", domain_name)],
        unlisted: false,
        redirect_to: None,
        removed_at: None,
        removed_if_alerted_after: if c.d >= 3 { Some(c.rng.int(10, 30) * 60) } else { None },
        keyword: true,
    };
    // página não indexada (painel de controle) — contém a expressão e a atribuição
    let panel = Page {
        url: format!("https://{}/painel", domain_name),
        title: "Painel (não indexado)".into(),
        body: format!("Página de controle não listada. Registro associado a {} ({}). Referência: {}.", suspect_name, c.suspect.email, k),
        links: vec![],
        unlisted: true,
        redirect_to: None,
        removed_at: None,
        removed_if_alerted_after: None,
        keyword: true,
    };
    let terms = Page {
        url: format!("https://{}/termos", domain_name),
        title: "Termos".into(),
        body: "Texto genérico de termos de uso.".into(),
        links: vec![format!("https://{}/", domain_name)],
        unlisted: false,
        redirect_to: None,
        removed_at: None,
        removed_if_alerted_after: None,
        keyword: false,
    };
    let pages = vec![home, panel.clone(), terms];
    case.keyword_sources.push(SourceRef::Page { url: panel.url.clone() });
    case.keyword_sources.push(SourceRef::Page { url: format!("https://{}/", domain_name) });
    case.keyword_sources.push(SourceRef::Archive { url: format!("https://{}/", domain_name) });

    // --- registro histórico (página capturada antes de sair do ar)
    let archive = vec![Snapshot {
        url: format!("https://{}/", domain_name),
        at: c.start - c.rng.int(2, 8) * 86400,
        title: format!("Acesso — {} (cópia histórica)", brand),
        body: format!("Cópia preservada da página em data anterior. Já continha a referência {}.", k),
        keyword: true,
    }];

    // --- e-mail de ameaça/golpe com trilha Received coerente
    let hop_internal = doc_ip(c, 0);
    let origin_ip = if c.d >= 3 { doc_ip(c, 2) } else { web_ip.clone() };
    let mail_time = c.start - c.rng.int(1, 5) * 86400;
    let received = vec![
        format!(
            "from mail.{dom} ({ptr} [{ip}]) by mx.correio.sim with ESMTPS id {rid}; {dt}",
            dom = domain_name,
            ptr = format!("mail-{}.hostprov.sim", mail_ip.replace('.', "-")),
            ip = mail_ip,
            rid = c.rng.hex(12),
            dt = fmt_rfc5322(mail_time + 2, BRT)
        ),
        format!(
            "from [{oip}] (origem) by mail.{dom} with SMTP id {rid}; {dt}",
            oip = origin_ip,
            dom = domain_name,
            rid = c.rng.hex(12),
            dt = fmt_rfc5322(mail_time, BRT)
        ),
    ];
    let mut extra = vec![
        ("X-Originating-IP".to_string(), format!("[{}]", origin_ip)),
        ("Authentication-Results".to_string(), format!("mx.correio.sim; spf=pass smtp.mailfrom={}; dkim=pass header.d={}", domain_name, domain_name)),
    ];
    if c.d >= 4 {
        // inconsistência: SPF pass mas o IP de origem não bate com o hop interno (ponto de atenção)
        extra.push(("X-Hop-Check".into(), format!("origem {} difere do servidor de saída {}", origin_ip, hop_internal)));
    }
    let mail = MailSpec {
        from: &format!("aviso@{}", domain_name),
        to: &c.victim.email,
        subject: &format!("[{}] mensagem recebida", case.keyword),
        body: &format!("Mensagem recebida pela vítima no âmbito do caso. Referência {}.", k),
        date: mail_time,
        offset: BRT,
        received,
        extra,
        message_id: format!("{}@{}", c.rng.hex(16), domain_name),
    };
    let eml = render_mail(&mail);
    let dg = c.rng.digits(4);
    let mut eml_file = c.file(&format!("mensagem_{}.eml", dg), FileType::Email, eml, mail_time, "requisitante");
    eml_file.keyword = true;
    case.provided.push(eml_file.clone());
    case.keyword_sources.push(SourceRef::Provided { file: eml_file.name().to_string() });

    // --- ISP e logs de conexão (CGNAT em dificuldade alta)
    let cgnat = c.d >= 3;
    let subscriber = format!("Contrato {} — {}", c.rng.digits(8), suspect_name);
    let priv_ip = format!("100.{}.{}.{}", c.rng.int(64, 127), c.rng.int(0, 255), c.rng.int(1, 254));
    let log_ip = origin_ip.clone();
    let conn_time = mail_time - 60;
    let logs = vec![CgnatRecord {
        public_ip: log_ip.clone(),
        port_start: if cgnat { 20000 } else { 0 },
        port_end: if cgnat { 21023 } else { 65535 },
        start: conn_time - 3600,
        end: conn_time + 7200,
        subscriber: subscriber.clone(),
        private_ip: if cgnat { priv_ip } else { log_ip.clone() },
    }];
    let isp = Isp {
        name: "Provedora RedeSul (fictícia)".into(),
        asn: "AS64510".into(),
        prefix: format!("{}.0/24", origin_ip.rsplit_once('.').map(|x| x.0).unwrap_or("203.0.113")),
        cgnat,
        logs,
    };

    case.net = Some(NetWorld {
        domains: vec![domain],
        dns,
        hosts,
        pages,
        archive,
        isps: vec![isp],
        cloud: None,
        expert_ips: expert_ips(),
    });

    // --- item físico: mídia com a mensagem/mandado encaminhada (material do caso)
    let mut item = base_item(c, "ITEM-01", ItemKind::Pendrive, "Mídia com material encaminhado para exame");
    item.description = "Pendrive contendo a mensagem eletrônica (.eml) e o histórico encaminhados pela autoridade.".into();
    item.media = vec![];
    seal_item(c, &mut item, 0.0);
    case.items.push(item);

    case.historico = format!(
        "A autoridade policial requisitou exame de local de internet no âmbito do {}, que apura {}. {} A investigação aponta {} como suspeito(a); a vítima {} encaminhou a mensagem eletrônica recebida.",
        case.procedimento, c.scenario.crime, c.scenario.summary, suspect_name, c.victim.name
    );
    case.objetivo = "Examinar vestígios de internet (domínio, página, mensagem eletrônica e registros de conexão), preservando o conteúdo e correlacionando as fontes, com atenção aos limites de atribuição.".into();
    case.quesitos = vec![
        "1. Identificar o domínio e os endereços associados (DNS, hospedagem).".into(),
        "2. Analisar o cabeçalho da mensagem eletrônica, verificando a trilha de servidores e a consistência de horários.".into(),
        "3. Preservar o conteúdo da página de interesse.".into(),
        format!("4. Correlacionar as fontes e localizar a expressão de interesse associada a {}, indicando a origem e as limitações de atribuição.", suspect_name),
    ];
    case.legal.mandado_texto = "Autorizada a realização de diligências em fontes abertas e a requisição de registros de conexão aos provedores. A identificação de pessoa depende dos registros do provedor e observa os limites legais.".into();
    case.briefing = vec![
        "Use uma conexão desvinculada da instituição para as diligências (o simulador oferece essa opção).".into(),
        "Um endereço IP identifica uma conexão técnica, não necessariamente uma pessoa — especialmente sob CGNAT.".into(),
        "Preserve as páginas e os cabeçalhos antes que saiam do ar; registre data, hora e o IP utilizado.".into(),
    ];
    if cgnat {
        case.briefing.push("Há indícios de CGNAT: a atribuição exige também a porta lógica de origem e o horário exato.".into());
    }
}
