//! Gerador de casos do POP 5.01 — exame de mídias de armazenamento.

use crate::casegen::*;
use crate::content::*;
use crate::model::*;

pub fn gen_501(c: &mut Ctx, case: &mut CaseFile) {
    let user = login(&c.suspect);
    let k = c.keyword.clone();
    case.historico = format!(
        "{} comunicou à autoridade policial a suspeita de {}. Durante cumprimento de mandado de busca e apreensão, foi arrecadado o material descrito abaixo, atribuído a {}. O material foi encaminhado lacrado a este laboratório para exame.",
        c.company.0, c.scenario.crime, c.suspect.name
    );
    case.objetivo = "Realizar exame pericial em mídia de armazenamento computacional, localizando e extraindo dados de interesse à investigação, com preservação da prova.".into();
    case.quesitos = vec![
        "1. Qual a descrição do material examinado?".into(),
        "2. Há arquivos relacionados aos fatos investigados, inclusive apagados? Em caso positivo, extraí-los e preservá-los.".into(),
        format!("3. Localiza-se a expressão de interesse da investigação associada a {}? Indicar a origem.", c.suspect.name),
        "4. Outros dados julgados úteis ao esclarecimento dos fatos.".into(),
    ];

    let (item_kind, label, mkind, cap) = match c.d {
        1 => (ItemKind::Hdd, "HD externo USB", MediaKind::Hdd, 500),
        2 => (ItemKind::Desktop, "Computador tipo desktop", MediaKind::Hdd, 1000),
        3 => (ItemKind::Notebook, "Notebook", MediaKind::Nvme, 512),
        _ => (ItemKind::Desktop, "Computador tipo desktop", MediaKind::Ssd, 480),
    };
    let mut item = base_item(c, "ITEM-01", item_kind, label);
    item.owner = Some(c.suspect.name.clone());
    item.physical.traces = vec!["impressões digitais latentes na superfície externa".into()];
    let mut m = new_media(c, "M-01", mkind, cap);
    if c.d == 1 {
        m.iface = "USB".into();
    }
    let mut p = partition(
        "M-01:p1",
        1,
        "NTFS",
        if c.d == 1 { "BACKUP" } else { "Windows" },
        m.capacity - 600_000_000,
        if c.d == 1 { 800 } else { 120_000 },
        &["/Windows/System32", "/Program Files", "/ProgramData"],
    );
    let suspect_name = c.suspect.name.clone();
    let mut files = windows_user_files(c, &user, &suspect_name);
    let kw_text = c.keyword_doc();
    let tk = c.t(2, 15);

    match c.d {
        1 => {
            let mut f = c.file(&format!("/Users/{}/Documents/Projetos/{}.docx", user, ascii_word(&k)), FileType::Documento, kw_text, tk, &user);
            f.keyword = true;
            case.keyword_sources.push(SourceRef::File { media: "M-01".into(), file: f.id.clone(), path: f.path.clone() });
            files.push(f);
        }
        2 => {
            let f = c.file(&format!("/Users/{}/Documents/entrega_final.docx", user), FileType::Documento, kw_text.clone(), tk, &user);
            let mut f = c.deleted(f, Recover::Integral);
            f.keyword = true;
            case.keyword_sources.push(SourceRef::File { media: "M-01".into(), file: f.id.clone(), path: f.path.clone() });
            files.push(f);
            let mut inner = c.file("entrega/resumo.txt", FileType::Texto, kw_text, tk, &user);
            inner.keyword = true;
            let mut zip = c.file(&format!("/Users/{}/Downloads/entrega.zip", user), FileType::Compactado, "[arquivo ZIP — use 'expandir' para listar o conteúdo]".into(), tk + 600, &user);
            case.keyword_sources.push(SourceRef::File { media: "M-01".into(), file: inner.id.clone(), path: format!("{}#{}", zip.path, inner.path) });
            zip.children.push(inner);
            files.push(zip);
            if c.rng.chance(0.4) {
                m.health = Health::SetoresRuins;
                m.bad_sectors = c.rng.int(12, 180) as u32;
                let i = c.rng.int(0, 2) as usize;
                files[i].bad_sector = true;
            }
        }
        _ => {
            let pw = bio_password(&mut c.rng, &c.suspect.clone());
            let mut inner_p = partition("M-01:vc1", 1, "FAT32", "VC", 2_000_000_000, 0, &[]);
            let mut f = c.file(&format!("/{}.docx", ascii_word(&k)), FileType::Documento, kw_text, tk, &user);
            f.keyword = true;
            let ff = c.file("/contatos.txt", FileType::Texto, format!("{} — contato inicial\n{} — intermediário", c.other.name, c.victim.name), tk - 3600, &user);
            inner_p.files = vec![f.clone(), ff];
            let mut cont = c.file(&format!("/Users/{}/Documents/backup_fotos.dat", user), FileType::ConteinerCifrado, "[dados de alta entropia — possível contêiner cifrado]".into(), tk + 3600, &user);
            cont.size = 2_000_000_000;
            cont.password = Some(pw.clone());
            cont.container = Some(Box::new(inner_p));
            cont.meta.insert("Entropia".into(), "7,99 bits/byte".into());
            case.keyword_sources.push(SourceRef::File { media: "M-01".into(), file: f.id.clone(), path: format!("{}#{}", cont.path, f.path) });
            files.push(cont);

            let old = c.t(60, 90);
            let mut decoy = c.file(&format!("/Users/{}/Documents/rascunho_antigo.txt", user), FileType::Texto, format!("Ideias iniciais — {} (descartado)", c.decoy), old, &user);
            decoy.meta.insert("Observação".into(), "versão anterior, não relacionada".into());
            files.push(decoy);
            case.decoys.push(c.decoy.clone());
            let gone = c.file(&format!("/Users/{}/Documents/lista_fornecedores.xlsx", user), FileType::Planilha, "Fornecedor;Contato\nAlfa Usinagem;(19) 3333-0000\n[...trecho sobrescrito...]".into(), old, &user);
            files.push(c.deleted(gone, Recover::Parcial));

            if c.d >= 3 {
                p.tz_offset = 0;
                p.dst_note = Some("Sistema configurado em UTC (fuso 00:00); horários exibidos pelo SO estão 3 h adiantados em relação ao horário de Brasília.".into());
            }
            if c.d == 4 {
                p.encrypted = Some(Encryption { scheme: "Cifragem de volume (simulada) vinculada ao equipamento".into(), password: format!("{}#{}", c.suspect.pet, c.rng.digits(4)), tpm: true });
                case.provided_passwords.push(("Senha de logon informada pelo investigado (consta no termo de apreensão)".into(), p.encrypted.as_ref().unwrap().password.clone()));
                let cred = format!("site;usuario;senha\nhttps://nuvem.arquivos.sim;{};{}", c.suspect.email, format!("{}{}", c.suspect.team.replace(' ', ""), c.rng.digits(3)));
                let tc = c.t(1, 5);
                files.push(c.file(&format!("/Users/{}/AppData/Local/Navegador/senhas_exportadas.csv", user), FileType::Config, cred, tc, &user));
                case.legal.autorizacao_nuvem = c.rng.chance(0.5);
                case.legal.mandado_texto = if case.legal.autorizacao_nuvem {
                    "Decisão judicial autoriza o acesso a dados armazenados, inclusive em serviços de nuvem vinculados às contas examinadas.".into()
                } else {
                    "Decisão judicial autoriza o acesso aos dados armazenados nas mídias apreendidas. NÃO há autorização para acesso a serviços de armazenamento em nuvem.".into()
                };
                let mut nf = c.file("/Compartilhado/negociacao.txt", FileType::Texto, format!("Tratativa referente ao {}. Valores a combinar.", k), tk, &user);
                nf.keyword = true;
                let cloud_pw = case_cloud_password(&files);
                case.keyword_sources.push(SourceRef::Cloud { path: nf.path.clone() });
                case.net = Some(NetWorld {
                    domains: vec![],
                    dns: vec![],
                    hosts: vec![],
                    pages: vec![],
                    archive: vec![],
                    isps: vec![],
                    cloud: Some(Cloud { provider: "nuvem.arquivos.sim".into(), account: c.suspect.email.clone(), password: cloud_pw, files: vec![nf] }),
                    expert_ips: expert_ips(),
                });
            }
        }
    }

    if c.d >= 2 {
        item.slots = vec![
            Slot { id: "SATA0".into(), label: "Porta SATA 0".into(), iface: m.iface.clone(), media_id: Some("M-01".into()), hidden_by: None, soldered: false },
            Slot { id: "SATA1".into(), label: "Porta SATA 1".into(), iface: "SATA".into(), media_id: None, hidden_by: None, soldered: false },
        ];
        if c.d == 3 {
            item.slots = vec![Slot { id: "M2_0".into(), label: "Slot M.2 (NVMe)".into(), iface: "NVMe".into(), media_id: Some("M-01".into()), hidden_by: None, soldered: false }];
        }
        item.components = vec![Component { id: "tampa".into(), kind: "tampa".into(), label: "Tampa lateral / inferior".into(), removed: false }];
    }
    item.media = vec!["M-01".into()];

    p.files = files;
    m.partitions.push(p);
    if c.d >= 3 && c.rng.chance(0.4) {
        m.health = Health::FalhaConector;
    }
    seal_item(c, &mut item, 0.35);
    item.description = format!("{} marca {}, modelo {}, nº de série {}", item.kind.label(), item.brand, item.model, item.serial);
    item.doc_description = Some(item.description.clone());
    case.items.push(item);
    case.media.push(m);

    case.briefing = vec![
        "Material recebido lacrado no laboratório. Conferir a documentação antes do deslacre.".into(),
        "A mídia original deve ser preservada: o exame se faz sobre a cópia (imagem forense).".into(),
    ];
    if c.d == 4 {
        let mref = case.media.iter().find(|m| m.id == "M-01").unwrap();
        case.documented_hashes.insert("M-01".into(), crate::vfs::media_hash(mref));
        case.briefing.push("Há um hash registrado em documento anterior; convém conferir após a aquisição.".into());
    }
    if !case.provided_passwords.is_empty() {
        case.briefing.push("O termo de apreensão informa senha fornecida pelo investigado.".into());
    }
}
