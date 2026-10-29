//! Geradores de casos dos POPs 5.02 (exame de móveis) e 5.05 (apreensão de móveis).

use crate::casegen::*;
use crate::content::*;
use crate::model::*;

fn imei() -> String {
    String::new()
}

fn make_phone(c: &mut Ctx, data_media: &str, afu: bool) -> (PhoneState, Vec<SourceRef>, Vec<VFile>) {
    let (brand, model, os) = *c.rng.pick(MOBILE_MODELS);
    let _ = imei();
    let imei1 = format!("35{}", c.rng.digits(13));
    let imei2 = format!("35{}", c.rng.digits(13));
    let two_sim = c.rng.chance(0.6);
    let op = c.rng.pick(OPERATORS);
    let pin = op.1.to_string();
    let sim = SimCard {
        iccid: format!("8955{}", c.rng.digits(15)),
        operadora: op.0.to_string(),
        pin: pin.clone(),
        puk: c.rng.digits(8),
        pin_tries: 3,
        puk_tries: 10,
        state: SimState::PinAtivo,
        place: SimWhere::Slot,
        slot: 1,
        contacts: vec![(c.victim.name.clone(), c.victim.phone.clone()), (c.other.name.clone(), c.other.phone.clone())],
        sms: vec![format!("{}: confirma amanhã?", first_name(&c.other)), "Operadora: seu plano foi renovado.".into()],
    };
    let mut sims = vec![sim];
    if two_sim {
        let op2 = c.rng.pick(OPERATORS);
        sims.push(SimCard {
            iccid: format!("8955{}", c.rng.digits(15)),
            operadora: op2.0.to_string(),
            pin: op2.1.to_string(),
            puk: c.rng.digits(8),
            pin_tries: 3,
            puk_tries: 10,
            state: SimState::PinAtivo,
            place: SimWhere::Slot,
            slot: 2,
            contacts: vec![],
            sms: vec![],
        });
    }
    let passcode = c.rng.digits(if c.d >= 3 { 6 } else { 4 });
    let encryption = if os == "iOS" || c.d >= 3 { "FBE" } else { "FDE" };
    let support_fs = match (os, afu, c.d) {
        ("iOS", true, _) => FsSupport::Afu,
        ("iOS", false, _) => FsSupport::Nao,
        (_, _, 1) => FsSupport::Sempre,
        (_, true, _) => FsSupport::Afu,
        (_, false, _) => FsSupport::Desbloqueado,
    };
    let ph = PhoneState {
        os: os.to_string(),
        os_version: format!("{}.{}", c.rng.int(11, 17), c.rng.int(0, 5)),
        imeis: vec![imei1.clone(), imei2.clone()],
        imei_label: vec![imei1, imei2],
        lock: if c.d == 1 { "nenhum".into() } else { "pin".into() },
        passcode,
        smartlock: if c.d == 2 && c.rng.chance(0.4) { Some("bluetooth".into()) } else { None },
        boot: if c.d == 1 { Boot::Desbloqueado } else if afu { Boot::Afu } else { Boot::Bfu },
        battery: c.rng.int(20, 90) as f64,
        charging: false,
        battery_removable: false,
        battery_removed: false,
        airplane: false,
        wifi: true,
        bt: true,
        cellular: true,
        rf: Rf::Nenhum,
        external_battery: false,
        sims,
        esim: if c.d >= 3 && c.rng.chance(0.4) { Some((format!("8955{}", c.rng.digits(15)), c.rng.pick(OPERATORS).0.to_string())) } else { None },
        sd_media: None,
        sd_inside: true,
        encryption: encryption.to_string(),
        support_logica: true,
        support_fs,
        support_fisica: c.d <= 2 && os == "Android",
        alarms: vec!["06:30 (dias úteis)".into()],
        alarms_disabled: false,
        remote_wipe_at: if c.d >= 2 { Some(c.start + c.rng.int(20, 90) * 60) } else { None },
        wiped: false,
        wipe_after_fails: if c.d == 4 { Some(10) } else { None },
        failed_unlocks: 0,
        second_space: None,
        front_camera_covered: false,
        sticker_on_screen: false,
        screen_broken: c.d >= 3 && c.rng.chance(0.3),
        port_damaged: false,
        data_media: data_media.to_string(),
        model: format!("{} {}", brand, model),
        users_known: vec![c.suspect.name.clone()],
    };
    (ph, vec![], vec![])
}

fn phone_data(c: &mut Ctx, pid: &str) -> (Partition, Vec<SourceRef>, bool) {
    let mut p = partition(pid, 1, "F2FS", "userdata", 64_000_000_000, 20_000, &["/system", "/data/app", "/data/dalvik-cache"]);
    let k = c.keyword.clone();
    let mut srcs = vec![];
    let tk = c.t(1, 10);

    // conversas do aplicativo de mensagens
    let conv = format!(
        "{}\n{}\n{}\n{}",
        chat_line(tk - 7200, first_name(&c.suspect), "bom dia"),
        chat_line(tk - 7000, first_name(&c.other), &format!("já viu o {}?", k)),
        chat_line(tk - 6800, first_name(&c.suspect), "vi sim, guardei aqui"),
        chat_line(tk - 6600, first_name(&c.other), "beleza"),
    );
    let mut chat = c.file("/data/data/sim.mensageiro/db/chat.txt", FileType::Chat, conv, tk, &c.suspect.name.clone());
    chat.keyword = true;
    srcs.push(SourceRef::File { media: c.suspect_media(pid), file: chat.id.clone(), path: chat.path.clone() });
    let _ = &mut p;

    // galeria com metadados de localização
    let tg = c.t(5, 40);
    let mut img = c.file("/data/media/0/DCIM/Camera/IMG_viagem.jpg", FileType::Imagem, "[imagem — EXIF disponível]".into(), tg, &c.suspect.name.clone());
    img.meta.insert("GPSLatitude".into(), format!("-{}.{}", c.rng.int(20, 27), c.rng.digits(4)));
    img.meta.insert("GPSLongitude".into(), format!("-{}.{}", c.rng.int(40, 51), c.rng.digits(4)));
    img.meta.insert("DataOriginal".into(), crate::util::fmt_dt(tg, crate::util::BRT));

    // registro de chamadas / arquivo apagado recuperável
    let tc = c.t(1, 6);
    let calls = format!("tipo;numero;inicio;duracao_s\nsaida;{};{};145\nentrada;{};{};320", c.victim.phone, crate::util::fmt_iso(tc), c.other.phone, crate::util::fmt_iso(tc - 3600));
    let callf = c.file("/data/data/com.android.dialer/call_log.csv", FileType::BancoDados, calls, tc, "system");

    let gone = c.file("/data/media/0/Download/comprovante.txt", FileType::Texto, format!("Registro associado ao {}.", k), tk - 1000, &c.suspect.name.clone());
    let mut goner = c.deleted(gone, if c.d >= 3 { Recover::Parcial } else { Recover::Integral });
    goner.keyword = c.d < 3;
    if goner.keyword {
        srcs.push(SourceRef::File { media: c.suspect_media(pid), file: goner.id.clone(), path: goner.path.clone() });
    }

    let mut files = vec![chat, img, callf, goner];

    // segundo espaço / cofre protegido (dificuldade alta)
    if c.d == 4 && c.rng.chance(0.7) {
        let pw = bio_password(&mut c.rng, &c.suspect.clone());
        let kd = c.keyword_doc();
        let sn = c.suspect.name.clone();
        let mut vaultf = c.file("/data/secure/cofre/anotacao.txt", FileType::Texto, kd, tk, &sn);
        vaultf.keyword = true;
        srcs.push(SourceRef::File { media: c.suspect_media(pid), file: vaultf.id.clone(), path: format!("/data/secure/cofre.img#{}", vaultf.path) });
        let mut vp = partition(&format!("{}:cofre", pid), 2, "ext4", "cofre", 1_000_000_000, 0, &[]);
        vp.files = vec![vaultf];
        vp.encrypted = Some(Encryption { scheme: "Espaço seguro do app (simulado)".into(), password: pw, tpm: false });
        p.files = files;
        return (combine(p, vp), srcs, true);
    }

    files.push(c.file("/data/system/packages.list", FileType::Sistema, "sim.mensageiro\ncom.android.dialer\nsim.banco".into(), tk, "system"));
    p.files = files;
    (p, srcs, true)
}

fn combine(mut a: Partition, b: Partition) -> Partition {
    // embute a partição do cofre como "arquivo contêiner" dentro da userdata
    let mut holder = VFile {
        id: format!("{}-cofre", a.id),
        path: "/data/secure/cofre.img".into(),
        size: b.size,
        mtime: a.files.first().map(|f| f.mtime).unwrap_or(0),
        ctime: 0,
        atime: 0,
        owner: "system".into(),
        ftype: FileType::ConteinerCifrado,
        deleted: false,
        recover: None,
        bad_sector: false,
        content: "[espaço seguro do aplicativo — protegido por senha]".into(),
        ocr: None,
        meta: Default::default(),
        password: b.encrypted.as_ref().map(|e| e.password.clone()),
        children: vec![],
        container: Some(Box::new(b)),
        keyword: false,
    };
    holder.meta.insert("Tipo".into(), "espaço seguro".into());
    a.files.push(holder);
    a
}

impl Ctx {
    fn suspect_media(&self, pid: &str) -> String {
        pid.split(':').next().unwrap_or(pid).to_string()
    }
}

pub fn gen_502(c: &mut Ctx, case: &mut CaseFile) {
    let afu = c.d <= 3 && c.rng.chance(if c.d == 1 { 1.0 } else { 0.5 });
    let mut m = new_media(c, "M-01", MediaKind::FlashMovel, 64);
    let (mut p, srcs, _) = phone_data(c, "M-01:data");
    p.tz_offset = crate::util::BRT;
    m.partitions.push(p);
    let (mut phone, _, _) = make_phone(c, "M-01", afu);
    if !afu && c.d == 1 {
        phone.boot = Boot::Desbloqueado;
    }

    let mut item = base_item(c, "ITEM-01", ItemKind::Celular, "Aparelho de telefonia celular");
    item.owner = Some(c.suspect.name.clone());
    let (b, mdl) = phone.model.split_once(' ').unwrap_or((&phone.model, ""));
    item.brand = b.to_string();
    item.model = mdl.to_string();
    item.phone = Some(phone.clone());
    item.media = vec!["M-01".into()];
    item.power_on = phone.boot != Boot::Desligado;
    if phone.screen_broken {
        item.physical.damage.push("tela frontal trincada (toque parcialmente funcional)".into());
    }
    item.physical.traces = vec!["aparelho com película e capa".into()];
    seal_item(c, &mut item, 0.3);
    item.description = format!("Aparelho celular {}, IMEI {}, nº de série {}", phone.model, phone.imeis[0], item.serial);
    item.doc_description = Some(item.description.clone());

    // condição de recebimento: o POP 5.02 exige continuar o isolamento iniciado na apreensão
    if c.d >= 2 {
        item.phone.as_mut().unwrap().rf = Rf::Nenhum;
        case.briefing.push("Atenção: o aparelho chegou ao laboratório LIGADO. Avalie o isolamento de comunicações antes de qualquer manuseio.".into());
    }

    case.historico = format!(
        "No âmbito do {}, apurando {}, foi apreendido o aparelho celular atribuído a {} e encaminhado para exame.",
        case.procedimento, c.scenario.crime, c.suspect.name
    );
    case.objetivo = "Realizar exame pericial no aparelho de telefonia celular, extraindo e analisando os dados de interesse à investigação, respeitando as limitações de acesso.".into();
    case.quesitos = vec![
        "1. Qual a descrição do aparelho (marca, modelo, IMEI, linha)?".into(),
        "2. Há registros de conversas, chamadas, arquivos ou dados de aplicativos relacionados aos fatos?".into(),
        format!("3. Localiza-se a expressão de interesse associada a {}? Indicar a origem e a forma de obtenção.", c.suspect.name),
        "4. Esclarecer o alcance e as limitações da extração realizada.".into(),
    ];
    case.keyword_sources = srcs;
    if c.d >= 2 {
        item.flags.insert("recebido_ligado".into(), "sim".into());
        let pc = item.phone.as_ref().unwrap().passcode.clone();
        case.provided_passwords.push(("PIN de desbloqueio informado pelo proprietário (termo de apreensão)".into(), pc));
    }
    case.items.push(item);
    case.media.push(m);
    case.briefing.insert(0, "Confirme o estado do aparelho (ligado/desligado, bloqueado/desbloqueado, AFU/BFU) antes de decidir o método de extração.".into());
    if afu {
        case.briefing.push("Dispositivo em estado AFU: mantê-lo isolado e carregando preserva a possibilidade de extração mais completa.".into());
    }
}

pub fn gen_505(c: &mut Ctx, case: &mut CaseFile) {
    // Apreensão: o dispositivo ainda está em campo, no momento da arrecadação.
    case.setting = Setting::Local;
    let afu = c.rng.chance(0.5);
    let mut m = new_media(c, "M-01", MediaKind::FlashMovel, 64);
    let (p, _srcs, _) = phone_data(c, "M-01:data");
    m.partitions.push(p);
    let (mut phone, _, _) = make_phone(c, "M-01", afu);
    phone.rf = Rf::Nenhum;
    phone.airplane = false;
    phone.sticker_on_screen = false;
    phone.front_camera_covered = false;
    if c.rng.chance(0.5) {
        phone.alarms.push("07:00 (todos os dias)".into());
    }
    if c.d >= 3 {
        phone.second_space = Some(SecondSpace { name: "espaço seguro".into(), password: bio_password(&mut c.rng, &c.suspect.clone()), media_partition: "M-01:cofre".into() });
    }

    let mut item = base_item(c, "ITEM-01", ItemKind::Celular, "Aparelho de telefonia celular");
    item.owner = Some(c.suspect.name.clone());
    let (b, mdl) = phone.model.split_once(' ').unwrap_or((&phone.model, ""));
    item.brand = b.to_string();
    item.model = mdl.to_string();
    item.power_on = true;
    // dano/condição especial conforme dificuldade
    if c.d == 4 {
        match c.rng.int(0, 2) {
            0 => item.physical.wet = true,
            1 => item.physical.battery_swollen = true,
            _ => item.physical.damage.push("tela quebrada".into()),
        }
    }
    item.phone = Some(phone.clone());
    item.media = vec!["M-01".into()];
    item.description = format!("Aparelho celular {}", phone.model);

    case.historico = format!(
        "Durante cumprimento de mandado no endereço de {}, no âmbito do {}, foi localizado o aparelho celular descrito, a ser apreendido e documentado.",
        c.suspect.name, case.procedimento
    );
    case.objetivo = "Apreender e documentar o dispositivo móvel, preservando seu estado e registrando as condições para posterior exame pericial.".into();
    case.quesitos = vec![];
    case.required_docs = vec![DocKind::TermoApreensao];
    case.keyword = String::new(); // apreensão não envolve busca de palavra-chave
    case.keyword_sources = vec![];
    case.items.push(item);
    case.media.push(m);
    case.briefing = vec![
        "Esta é uma etapa de APREENSÃO, não de exame. O objetivo é preservar e documentar — não analisar o conteúdo.".into(),
        "Reconheça, isole e fixe (fotografe) o dispositivo antes de manuseá-lo.".into(),
        "Solicite senhas ao proprietário e registre-as no termo; verifique o estado de energia e bloqueio.".into(),
    ];
    if afu {
        case.briefing.push("Dispositivo em AFU: planeje o isolamento RF (Faraday bag ou ≥ 3 camadas de alumínio) e a alimentação (bateria externa).".into());
    }
}
