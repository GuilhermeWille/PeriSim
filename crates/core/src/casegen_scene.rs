//! Gerador de casos do POP 5.03 — exame de local de informática.
//! O cenário é um ambiente (sala/escritório) com equipamentos a inspecionar.

use crate::casegen::*;
use crate::content::*;
use crate::model::*;

pub fn gen_503(c: &mut Ctx, case: &mut CaseFile) {
    case.setting = Setting::Local;
    let user = login(&c.suspect);
    let k = c.keyword.clone();
    let suspect_name = c.suspect.name.clone();

    case.historico = format!(
        "Cumpre-se mandado de busca e apreensão no endereço vinculado a {}, no âmbito do {}, apurando {}. O perito foi acionado para examinar o local de informática.",
        suspect_name, case.procedimento, c.scenario.crime
    );
    case.objetivo = "Examinar o local de informática, identificando, preservando e coletando os vestígios digitais relevantes, com a documentação do ambiente.".into();
    case.quesitos = vec![
        "1. Descrever os equipamentos computacionais encontrados no local.".into(),
        "2. Preservar e coletar dados voláteis e não voláteis de interesse, quando pertinente.".into(),
        format!("3. Localizar vestígios relacionados aos fatos, inclusive a expressão de interesse associada a {}.", suspect_name),
        "4. Documentar as condições do ambiente e as limitações da coleta.".into(),
    ];

    // --- ITEM-01: computador LIGADO, com volume cifrado montado (dados voláteis relevantes)
    let mut desktop = base_item(c, "ITEM-01", ItemKind::Desktop, "Computador tipo desktop (sobre a mesa)");
    desktop.owner = Some(suspect_name.clone());
    desktop.power_on = true;
    desktop.physical.traces = vec!["ligado, monitor aceso".into()];
    desktop.components = vec![
        Component { id: "tampa".into(), kind: "tampa".into(), label: "Tampa lateral".into(), removed: false },
        Component { id: "gpu".into(), kind: "gpu".into(), label: "Placa de vídeo".into(), removed: false },
    ];

    let mut m1 = new_media(c, "M-01", if c.d >= 3 { MediaKind::Nvme } else { MediaKind::Ssd }, if c.d >= 3 { 512 } else { 480 });
    let mut p1 = partition("M-01:p1", 1, "NTFS", "Windows", m1.capacity - 500_000_000, 120_000, &["/Windows", "/Program Files"]);
    let tk = c.t(2, 20);
    let mut files = windows_user_files(c, &user, &suspect_name);

    // volume cifrado: aberto agora (na RAM), fechado após desligar
    let mut cont_part = partition("M-01:enc", 2, "exFAT", "COFRE", 8_000_000_000, 0, &[]);
    let kd = c.keyword_doc();
    let mut kwf = c.file(&format!("/{}.docx", ascii_word(&k)), FileType::Documento, kd, tk, &user);
    kwf.keyword = true;
    cont_part.files = vec![kwf.clone()];
    cont_part.encrypted = Some(Encryption { scheme: "Volume cifrado montado (simulado)".into(), password: bio_password(&mut c.rng, &c.suspect.clone()), tpm: false });
    let mut cont = c.file("/Users/".to_string().as_str(), FileType::ConteinerCifrado, "[volume cifrado]".into(), tk, &user);
    cont.path = format!("/Users/{}/Documents/cofre.vc", user);
    cont.size = 8_000_000_000;
    cont.password = cont_part.encrypted.as_ref().map(|e| e.password.clone());
    cont.container = Some(Box::new(cont_part));
    files.push(cont);
    p1.files = files;
    m1.partitions.push(p1);

    // estado vivo: volume montado + conexão de rede + processo suspeito
    let wipe_at = c.start + c.rng.int(15, 60) * 60;
    desktop.live = Some(LiveState {
        logged_user: user.clone(),
        processes: vec![
            ("explorer.exe".into(), 1840, String::new()),
            ("mensageiro.exe".into(), 3920, "aplicativo de mensagens aberto".into()),
            ("cofre-cript.exe".into(), 4510, "software de cifragem — volume montado".into()),
        ],
        mounted_encrypted: Some(("M-01:enc".into(), "V:".into())),
        ram_artifacts: vec![
            ("chave-volume".into(), "Chave do volume cifrado reside apenas na memória enquanto montado.".into(), false),
            ("conversa".into(), format!("Trecho de conversa recente mencionando {}.", k), true),
        ],
        screen: "Área de trabalho; janela do aplicativo de mensagens aberta; unidade V: montada.".into(),
        network_connected: true,
        remote_access: if c.d >= 3 { Some(("ferramenta de acesso remoto".into(), wipe_at)) } else { None },
        memory_dump: None,
    });
    case.keyword_sources.push(SourceRef::File { media: "M-01".into(), file: kwf.id.clone(), path: format!("/Users/{}/Documents/cofre.vc#{}", user, kwf.path) });
    case.keyword_sources.push(SourceRef::Ram { item: "ITEM-01".into() });
    if c.d >= 3 {
        case.events.push(ScheduledEvent {
            at: wipe_at,
            kind: "remote-access-wipe".into(),
            target: "ITEM-01".into(),
            note: "Comando de apagamento remoto chega se o equipamento permanecer conectado à rede.".into(),
            fired: false,
        });
    }
    desktop.media = vec!["M-01".into()];
    seal_item_scene(c, &mut desktop);
    desktop.description = format!("{} marca {}, nº de série {}", desktop.kind.label(), desktop.brand, desktop.serial);
    case.items.push(desktop);
    case.media.push(m1);

    // --- ITEM-02: computador DESLIGADO (não deve ser iniciado pelo SO instalado)
    let mut off = base_item(c, "ITEM-02", ItemKind::Desktop, "Computador tipo desktop (desligado, sob a mesa)");
    off.owner = Some(suspect_name.clone());
    off.power_on = false;
    let mut m2 = new_media(c, "M-02", MediaKind::Hdd, 1000);
    let mut p2 = partition("M-02:p1", 1, "NTFS", "Dados", m2.capacity - 400_000_000, 90_000, &["/Windows"]);
    let old = c.t(30, 120);
    let mut f2 = c.file("/Arquivos/backup/anotacoes.txt", FileType::Texto, format!("Backup antigo. Menção ao {} em rascunho.", c.decoy), old, &user);
    f2.meta.insert("Observação".into(), "material anterior aos fatos".into());
    case.decoys.push(c.decoy.clone());
    p2.files = vec![f2];
    m2.partitions.push(p2);
    off.media = vec!["M-02".into()];
    off.slots = vec![Slot { id: "SATA0".into(), label: "Porta SATA 0".into(), iface: "SATA".into(), media_id: Some("M-02".into()), hidden_by: None, soldered: false }];

    // mídia oculta: pendrive camuflado / SSD sob o dissipador (POP 5.03 §4.4)
    if c.d >= 2 {
        let mut m3 = new_media(c, "M-03", if c.d == 4 { MediaKind::Nvme } else { MediaKind::Usb }, if c.d == 4 { 256 } else { 32 });
        let mut p3 = partition("M-03:p1", 1, "exFAT", "KINGS", m3.capacity - 100_000_000, 40, &[]);
        let tk2 = c.t(1, 10);
        let kd2 = c.keyword_doc();
        let mut kf = c.file(&format!("/{}_copia.docx", ascii_word(&k)), FileType::Documento, kd2, tk2, &user);
        kf.keyword = true;
        p3.files = vec![kf.clone()];
        m3.partitions.push(p3);
        if c.d == 4 {
            // SSD M.2 escondido sob o dissipador do ITEM-02
            off.components.push(Component { id: "dissipador".into(), kind: "dissipador".into(), label: "Dissipador de calor (cooler)".into(), removed: false });
            off.components.push(Component { id: "tampa2".into(), kind: "tampa".into(), label: "Tampa lateral".into(), removed: false });
            off.slots.push(Slot { id: "M2_oculto".into(), label: "Slot M.2 oculto".into(), iface: "NVMe".into(), media_id: Some("M-03".into()), hidden_by: Some("dissipador".into()), soldered: false });
            off.media.push("M-03".into());
            off.physical.camouflaged = Some("SSD M.2 instalado sob o dissipador, fora do slot principal".into());
        } else {
            // pendrive camuflado como chaveiro, em uma gaveta
            let mut flash = base_item(c, "ITEM-03", ItemKind::Pendrive, "Pendrive camuflado (formato de chaveiro)");
            flash.owner = Some(suspect_name.clone());
            flash.hidden = true;
            flash.physical.camouflaged = Some("pendrive embutido em chaveiro de borracha, em gaveta".into());
            flash.media = vec!["M-03".into()];
            flash.description = "Pendrive dissimulado em chaveiro".into();
            seal_item_scene(c, &mut flash);
            case.items.push(flash);
        }
        case.media.push(m3);
        case.keyword_sources.push(SourceRef::File { media: "M-03".into(), file: kf.id.clone(), path: kf.path.clone() });
    }
    seal_item_scene(c, &mut off);
    off.description = format!("{} marca {}, nº de série {}", off.kind.label(), off.brand, off.serial);
    case.items.push(off);
    case.media.push(m2);

    case.briefing = vec![
        "Local de informática: documente o ambiente antes de intervir.".into(),
        "Há um equipamento LIGADO e outro DESLIGADO — o tratamento difere para cada um.".into(),
        "Dados voláteis podem conter chaves de cifragem que se perdem ao desligar.".into(),
    ];
    if c.d >= 2 {
        case.briefing.push("Procure mídias que possam estar fisicamente ocultas ou camufladas.".into());
    }
}

fn seal_item_scene(c: &mut Ctx, it: &mut Item) {
    // No local, o lacre é aposto pelo perito durante a coleta; o item chega sem lacre.
    it.doc_seal = None;
    it.seal = None;
    let _ = c;
}
