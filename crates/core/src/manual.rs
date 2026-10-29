//! Manual procedimental resumido (exibido no simulador) e matriz de rastreabilidade.
//! Fonte: POP Perícia Criminal 2024 — Informática Forense, vol. 5 (SENASP/MJSP).
//! Os resumos são PARÁFRASES para estudo; itens marcados "[complemento]" não vêm do POP.

use crate::model::ModuleId;

pub fn pop_summary(m: ModuleId) -> &'static str {
    match m {
        ModuleId::M501 => "\
POP 5.01 — EXAME DE MÍDIAS DE ARMAZENAMENTO (p. 13–21)

Recebimento
 • Atenção a vestígios físicos (ex.: impressões digitais) antes do manuseio.
 • Conferir a descrição do material e o número do lacre com a documentação ANTES
   do deslacre; fotografar o material lacrado; registrar divergências.
 • Deslacrar e reconferir; remover a mídia do equipamento quando viável,
   registrando a interface/porta em que estava conectada.

Preservação e aquisição
 • O exame é feito sobre a CÓPIA; o original só é usado se a cópia for inviável.
 • Usar bloqueador de escrita (hardware ou software) ou equipamento forense.
 • Preferir duplicação mídia-para-arquivo-imagem; verificar o hash da cópia.
 • Cifragem dependente do equipamento: espelhamento mídia-para-mídia, com
   sanitização prévia da mídia de destino.

Processamento e análise
 • Recuperação de apagados, indexação, categorização, hash, detecção de
   cifragem, expansão de arquivos compostos, OCR.
 • Cifragem: recuperação direta da senha, tabelas, força bruta ou dicionário
   com regras (dados biográficos do usuário ajudam a montar o dicionário).

Pontos críticos
 • RAID/JBOD; ordem de boot na BIOS; credenciais de nuvem só com autorização
   judicial; fuso horário e horário de verão do sistema examinado.

Laudo
 • Identificar o material, distinguir original × cópia, registrar recuperação,
   softwares usados, alterações, hash da lista de hashes do anexo e novo lacre.",
        ModuleId::M502 => "\
POP 5.02 — EXAME DE DISPOSITIVOS MÓVEIS (p. 25–33)

 • Isolar comunicações (gaiola/sala de Faraday ou bloqueador de sinais).
 • Remover o SIM registrando o slot; cartão microSD removido apenas para
   identificação/duplicação e reinserido.
 • Estado AFU: manter isolado e carregando (preserva extração mais completa).
 • eSIM: ligar o aparelho dentro do isolamento.
 • Modo avião e verificação de todas as conexões, quando acessível.
 • Desbloqueio por ferramenta ou dicionário — risco de restauração de fábrica
   após tentativas; PIN do SIM bloqueia após 3 tentativas e PUK após 10.
 • Desabilitar alarmes.
 • Extração mais completa possível (física × sistema de arquivos × lógica),
   conforme o tipo de criptografia (FBE/FDE) e o estado do aparelho.
 • Em AFU, verificar a integridade da extração antes de desligar.
 • Número da linha e correio de voz dependem da operadora.
 • IMEI físico × lógico; atenção a segundo espaço e cofres de aplicativos.
 • Laudo: IMEIs, ICCID, operadora, método e limitações da extração.",
        ModuleId::M503 => "\
POP 5.03 — EXAME DE LOCAL DE INFORMÁTICA (p. 37–45)

Ações preparatórias e no local
 • Levantamento prévio; verificar restrições legais/mandado.
 • Reconhecer os equipamentos; isolamento físico e digital; fixação fotográfica.

Equipamento LIGADO
 • Respeitar a ordem de volatilidade; dump de memória quando houver interesse
   (ex.: suspeita de cifragem); registrar programas em execução.
 • Partição cifrada montada → copiar o conteúdo antes de desligar.
 • Para apreensão, desligar por corte de energia.
 • Partição cifrada → apreender o equipamento completo.
 • Servidor: auxílio do responsável de TI (verificando se não é suspeito).

Equipamento DESLIGADO
 • Não iniciar o sistema instalado: usar bloqueador ou boot forense e conferir
   a ordem de boot na BIOS.

Coleta
 • Procurar mídias camufladas ou ocultas (ex.: SSD sob dissipador/placa de vídeo).
 • Registrar hashes e lacres no termo; embalagem antiestática.",
        ModuleId::M504 => "\
POP 5.04 — EXAME DE LOCAL DE INTERNET (p. 49–55)

 • Usar conexão desvinculada da instituição; verificar autorização legal.
 • IP/domínio: WHOIS, traceroute, DNS reverso, geolocalização.
 • A atribuição depende do provedor; em CGNAT, também da porta lógica e do
   horário exato. Dados de WHOIS podem ser falsos.
 • E-mail: obter o cabeçalho completo; ler os campos Received em ordem inversa;
   verificar a consistência de horários e IP/domínio; open relay;
   X-Originating-IP; DKIM, SPF e ARC.
 • Sites: preservar no formato original, registrando data/hora e o IP usado;
   novas tentativas e consulta a sítios de histórico.
 • Nuvem: preferir a quebra judicial; captura direta com credenciais tem risco.
 • Priorizar pela volatilidade.
 [complemento] Endereço técnico, conta, dispositivo, conexão e indivíduo são
 entidades diferentes: o IP aponta uma conexão/assinante, não a pessoa.",
        ModuleId::M505 => "\
POP 5.05 — APREENSÃO DE DISPOSITIVOS MÓVEIS (p. 59–65)

 • Levar baterias externas carregadas.
 • Reconhecer, isolar e fixar (fotografar) o dispositivo; solicitar todos os
   aparelhos; o proprietário NÃO manipula o aparelho.
 • Etiqueta sobre a câmera frontal (nada mais sobre a tela).
 • Solicitar senhas (inclusive de segundo espaço/cofres); verificar smartlock;
   anotar senhas no termo e em etiqueta no verso.
 • Modo avião e registro de que foi (ou não) possível ativá-lo.
 • Desabilitar alarmes.
 • Sempre remover o SIM, registrando slot, operadora e ICCID; afixá-lo no verso.
 • Desligar se não houver interesse em AFU; bateria removível é retirada.
 • Cartões de memória relacionados e mantidos no aparelho.
 • AFU → Faraday bag ou ≥ 3 camadas de alumínio + bateria externa.
 • Embalar e lacrar individualmente; aparelho danificado, molhado ou
   contaminado → cuidados e etiquetas de alerta.
 • Estrutura de laudo: NÃO SE APLICA — o documento é o termo de apreensão.",
    }
}

/// Linha da matriz de rastreabilidade.
pub struct Trace {
    pub id: &'static str,
    pub fonte: &'static str,
    pub requisito: &'static str,
    pub comportamento: &'static str,
    pub teste: &'static str,
    pub limitacao: &'static str,
}

pub const MATRIX: &[Trace] = &[
    Trace { id: "R01", fonte: "POP 5.01 §4.1 (p.15)", requisito: "Conferir lacre/descrição antes do deslacre e registrar divergências", comportamento: "Comando `lacre conferir` compara lacre físico × documentação; divergência é registrada e cobrada no laudo", teste: "fluxo_501_d1_correto_aprova; avaliação c4", limitacao: "Divergência modelada como troca de dígitos" },
    Trace { id: "R02", fonte: "POP 5.01 §4.1", requisito: "Fotografar material lacrado", comportamento: "`fotografar`; deslacre sem foto gera falha", teste: "checklist 5.01", limitacao: "Foto é registro simbólico" },
    Trace { id: "R03", fonte: "POP 5.01 §4.2", requisito: "Exame sobre a cópia; original só se inviável", comportamento: "`fs montar` só aceita mídia adquirida", teste: "exame_sem_imagem_e_bloqueado_e_sem_bloqueador_e_penalizado", limitacao: "—" },
    Trace { id: "R04", fonte: "POP 5.01 §4.2", requisito: "Bloqueador de escrita", comportamento: "Aquisição sem `bloqueio` registra contaminação e reduz nota", teste: "exame_sem_imagem_e_bloqueado_e_sem_bloqueador_e_penalizado", limitacao: "Não simula alteração concreta de metadados" },
    Trace { id: "R05", fonte: "POP 5.01 §4.2", requisito: "Verificar hash da cópia", comportamento: "`imagem` calcula SHA-256; `verificar-hash` confere com recálculo e hash documentado", teste: "fluxo_501_d4_hash_documentado_confere; vetores_fips", limitacao: "Hash calculado sobre a representação canônica do modelo" },
    Trace { id: "R06", fonte: "POP 5.01 §4.2", requisito: "Cifragem dependente do equipamento → espelhamento com destino sanitizado", comportamento: "`imagem` recusa mídia FDE; `clone` exige `sanitizar`", teste: "(unitário de engine — pendente de caso gerado com FDE)", limitacao: "Geração atual não sorteia FDE por hardware" },
    Trace { id: "R07", fonte: "POP 5.01 §4.4", requisito: "Recuperação de apagados", comportamento: "`recuperar` integral/parcial/inviável conforme o modelo; setores ruins impedem", teste: "fluxo_501_d2_recuperacao_e_composto", limitacao: "Parcial = truncamento do conteúdo" },
    Trace { id: "R08", fonte: "POP 5.01 §4.4", requisito: "Expansão de arquivos compostos", comportamento: "Conteúdo de zip só acessível após `expandir`", teste: "fluxo_501_d2_recuperacao_e_composto", limitacao: "Um nível de aninhamento" },
    Trace { id: "R09", fonte: "POP 5.01 §4.4", requisito: "OCR", comportamento: "`ocr` revela texto de imagens; busca inclui OCR", teste: "—", limitacao: "Texto pré-definido no caso" },
    Trace { id: "R10", fonte: "POP 5.01 §4.5", requisito: "Senhas por dicionário com dados biográficos", comportamento: "`dicionario` gera candidatos; `decifrar` testa", teste: "fluxo_501_d3_conteiner_cifrado_por_dicionario", limitacao: "Sem força bruta temporal" },
    Trace { id: "R11", fonte: "POP 5.01 (pontos críticos)", requisito: "Fuso horário/horário de verão", comportamento: "Partição em UTC no D3+; `fs meta` alerta", teste: "avaliação c7 (fs-meta)", limitacao: "Sem horário de verão histórico" },
    Trace { id: "R12", fonte: "POP 5.01 (pontos críticos)", requisito: "Nuvem só com autorização judicial", comportamento: "D4 sorteia autorização; credenciais encontradas no disco", teste: "—", limitacao: "Acesso à nuvem ainda não exposto no terminal (pendente)" },
    Trace { id: "R13", fonte: "POP 5.02", requisito: "Isolar comunicações", comportamento: "`isolar` (Faraday/alumínio/sala/jammer); sem isolamento o comando remoto apaga o aparelho", teste: "aparelho_ligado_nao_isolado_e_apagado_remotamente", limitacao: "Momento do comando remoto sorteado" },
    Trace { id: "R14", fonte: "POP 5.02 / 5.05", requisito: "Remover SIM registrando slot, operadora, ICCID", comportamento: "`sim remover` registra na auditoria e no termo", teste: "fluxo_505_apreensao_gera_termo_nao_laudo", limitacao: "—" },
    Trace { id: "R15", fonte: "POP 5.02", requisito: "AFU: manter isolado e carregando", comportamento: "`carregar`; estado AFU habilita extração de sistema de arquivos", teste: "fluxo_502_isolado_extrai_e_encontra", limitacao: "Bateria não se esgota ainda (pendente)" },
    Trace { id: "R16", fonte: "POP 5.02", requisito: "Extração mais completa possível conforme criptografia/estado", comportamento: "`extrair` escolhe física > FS > lógica conforme suporte e estado", teste: "fluxo_502_isolado_extrai_e_encontra", limitacao: "Matriz de suporte simplificada" },
    Trace { id: "R17", fonte: "POP 5.02", requisito: "Risco de restauração após tentativas de senha", comportamento: "D4: 10 tentativas erradas apagam o aparelho", teste: "tentativas_de_senha_levam_a_apagamento_no_d4", limitacao: "Limite fixo" },
    Trace { id: "R18", fonte: "POP 5.02", requisito: "Segundo espaço / cofres", comportamento: "D4 pode conter cofre cifrado na partição do aparelho", teste: "toda_fonte_da_palavra_chave_existe_e_contem_a_expressao", limitacao: "—" },
    Trace { id: "R19", fonte: "POP 5.03 §4.3", requisito: "Ordem de volatilidade; dump de memória", comportamento: "`memoria` só com equipamento ligado; artefatos de RAM", teste: "fluxo_503_coleta_volatil_correta", limitacao: "Conteúdo da RAM é resumido" },
    Trace { id: "R20", fonte: "POP 5.03 §4.3", requisito: "Partição cifrada montada → copiar antes de desligar", comportamento: "`volume copiar`; desligar antes perde o acesso", teste: "desligar_com_volume_montado_perde_evidencia", limitacao: "—" },
    Trace { id: "R21", fonte: "POP 5.03 §4.2", requisito: "Isolamento digital", comportamento: "`rede isolar`; sem ele, acesso remoto apaga a RAM (D3+)", teste: "acesso_remoto_apaga_ram_se_nao_isolado", limitacao: "—" },
    Trace { id: "R22", fonte: "POP 5.03 §4.4", requisito: "Não iniciar SO de equipamento desligado; mídia em uso não é adquirida ligada", comportamento: "`imagem` recusa mídia de equipamento ligado", teste: "midia_em_uso_nao_pode_ser_imageada_ligada", limitacao: "Boot forense/BIOS não modelados" },
    Trace { id: "R23", fonte: "POP 5.03 §4.4", requisito: "Mídias camufladas/ocultas", comportamento: "`inspecionar` revela itens; `componente remover` revela slots ocultos", teste: "fluxo_503_coleta_volatil_correta", limitacao: "—" },
    Trace { id: "R24", fonte: "POP 5.03 §4.5", requisito: "Embalagem antiestática, lacres e hashes no termo", comportamento: "`embalar`, `lacre aplicar`; hashes no anexo", teste: "checklist 5.03", limitacao: "—" },
    Trace { id: "R25", fonte: "POP 5.04", requisito: "Conexão desvinculada da instituição", comportamento: "`conexao`; host monitorado + conexão não anônima alerta o investigado e derruba páginas", teste: "diligencia_sem_anonimato_em_host_monitorado_alerta_e_derruba_pagina", limitacao: "—" },
    Trace { id: "R26", fonte: "POP 5.04", requisito: "WHOIS/DNS/traceroute coerentes", comportamento: "Consultas sobre o mesmo modelo de rede", teste: "fluxo_504_rede_emulada_coerente; horarios_coerentes", limitacao: "Rota simplificada (4 saltos)" },
    Trace { id: "R27", fonte: "POP 5.04", requisito: "Cabeçalho completo, Received em ordem inversa, X-Originating-IP, SPF/DKIM", comportamento: "`email cabecalho` lista saltos da origem ao destino", teste: "fluxo_504_rede_emulada_coerente", limitacao: "ARC não modelado" },
    Trace { id: "R28", fonte: "POP 5.04", requisito: "Preservar página com data/hora/IP; sítios de histórico", comportamento: "`preservar` gera artefato com hash; `arquivo-web`", teste: "fluxo_504_rede_emulada_coerente", limitacao: "—" },
    Trace { id: "R29", fonte: "POP 5.04", requisito: "Atribuição depende do provedor; CGNAT exige porta", comportamento: "`rede logs` mostra porta/assinante e alerta que conexão ≠ pessoa", teste: "fluxo_504_rede_emulada_coerente; avaliação c8", limitacao: "—" },
    Trace { id: "R30", fonte: "POP 5.05 (p.63)", requisito: "Estrutura de laudo não se aplica; documento é o termo", comportamento: "Módulo 5.05 gera termo de apreensão, sem palavra-chave", teste: "fluxo_505_apreensao_gera_termo_nao_laudo", limitacao: "Campos do termo derivados dos itens do POP" },
    Trace { id: "R31", fonte: "Especificação + prompt", requisito: "Reprodutibilidade por semente e versão", comportamento: "ID do caso codifica módulo, dificuldade e semente", teste: "mesma_semente_reproduz_o_caso; id_do_caso_ida_e_volta", limitacao: "—" },
    Trace { id: "R32", fonte: "Prompt §11", requisito: "Coerência relatório × auditoria", comportamento: "check_coherence aponta procedimentos alegados e não realizados e conclusões categóricas", teste: "relatorio_incoerente_com_historico_e_detectado", limitacao: "Heurística por palavras-chave" },
    Trace { id: "R33", fonte: "Prompt §8", requisito: "Isolamento de rede e sem execução de comandos do SO", comportamento: "Motor sem bibliotecas de rede/processo", teste: "motor_nao_usa_rede_nem_processos; terminal_nao_executa_comandos_do_sistema", limitacao: "Verificação estática do código-fonte" },
    Trace { id: "R34", fonte: "Prompt §10", requisito: "Palavra-chave não compensa destruição", comportamento: "Perda de evidência limita a nota a 60%", teste: "palavra_chave_por_acaso_nao_compensa_destruicao", limitacao: "—" },
    Trace { id: "R35", fonte: "POP 5.02 / 5.05", requisito: "AFU: manter isolado E carregando", comportamento: "Bateria descarrega com o tempo (mais rápido isolado procurando sinal); ao zerar, desliga e perde o estado AFU", teste: "bateria_esgota_em_afu_sem_alimentacao_e_perde_estado; bateria_externa_preserva_afu", limitacao: "Taxas de descarga ilustrativas" },
    Trace { id: "R36", fonte: "POP 5.02", requisito: "IMEI físico × lógico", comportamento: "`imei verificar` compara etiqueta com *#06#; divergência (D4) deve constar no laudo", teste: "imei_fisico_x_logico_detecta_divergencia", limitacao: "—" },
    Trace { id: "R37", fonte: "POP 5.02 (eSIM) / 5.05", requisito: "Ligar o aparelho somente dentro do isolamento; desligar só sem interesse em AFU", comportamento: "`energia ligar|desligar` em móveis: ligar fora do isolamento é falha; desligar em AFU antes da extração descarta o estado", teste: "ligar_celular_fora_do_isolamento_e_falha_e_desligar_em_afu_descarta_estado", limitacao: "—" },
    Trace { id: "R38", fonte: "POP 5.01 §4.2 / 5.03", requisito: "Exame sobre a cópia, somente leitura; sem montar o original", comportamento: "Linux da estação: imagens montadas `ro`; montar o dispositivo original é registrado e, sem bloqueador, contamina", teste: "exame_501_completo_pelo_terminal_linux; montar_original_e_registrado_e_sem_bloqueador_contamina", limitacao: "Shell é um subconjunto do bash" },
    Trace { id: "R39", fonte: "POP 5.02 / 5.05", requisito: "Navegação manual na tela só como complemento documentado; na apreensão, não examinar o conteúdo", comportamento: "Tela do aparelho (FoneOS): cada app aberto vira `tela <item> <app>` na auditoria; navegar antes da extração altera o estado (mensagens lidas) e é falta; em 5.05, abrir conteúdo é falta, configurações/alarmes/IMEI não; `tela <item> foto` documenta", teste: "navegar_antes_da_extracao_e_falta_e_marca_mensagens_como_lidas; navegar_depois_da_extracao_e_complemento_documentado; apreensao_permite_configuracoes_mas_nao_conteudo", limitacao: "Conteúdo exibido deriva do modelo do caso; sem teclado de desbloqueio na tela" },
    Trace { id: "R40", fonte: "POP 5.02 / 5.05", requisito: "Tela bloqueada não expõe conteúdo; IMEI pelo código de serviço", comportamento: "BFU: sem notificações; AFU: contagem sem conteúdo; discador de emergência com *#06# (exige ligado)", teste: "bloqueado_nao_expoe_conteudo_pela_tela; imei_pela_tela_usa_o_codigo_de_servico; desligado_mostra_tela_apagada", limitacao: "Modelo didático genérico, não reproduz um sistema real" },
];

pub fn matrix_markdown() -> String {
    let mut s = String::from("# Matriz de rastreabilidade\n\nGerada a partir de `crates/core/src/manual.rs` (fonte única).\n\n| ID | Fonte | Requisito | Comportamento no simulador | Teste | Limitação |\n|---|---|---|---|---|---|\n");
    for t in MATRIX {
        s.push_str(&format!("| {} | {} | {} | {} | {} | {} |\n", t.id, t.fonte, t.requisito, t.comportamento, t.teste, t.limitacao));
    }
    s
}
