# Matriz de rastreabilidade

Gerada a partir de `crates/core/src/manual.rs` (fonte única).

| ID | Fonte | Requisito | Comportamento no simulador | Teste | Limitação |
|---|---|---|---|---|---|
| R01 | POP 5.01 §4.1 (p.15) | Conferir lacre/descrição antes do deslacre e registrar divergências | Comando `lacre conferir` compara lacre físico × documentação; divergência é registrada e cobrada no laudo | fluxo_501_d1_correto_aprova; avaliação c4 | Divergência modelada como troca de dígitos |
| R02 | POP 5.01 §4.1 | Fotografar material lacrado | `fotografar`; deslacre sem foto gera falha | checklist 5.01 | Foto é registro simbólico |
| R03 | POP 5.01 §4.2 | Exame sobre a cópia; original só se inviável | `fs montar` só aceita mídia adquirida | exame_sem_imagem_e_bloqueado_e_sem_bloqueador_e_penalizado | — |
| R04 | POP 5.01 §4.2 | Bloqueador de escrita | Aquisição sem `bloqueio` registra contaminação e reduz nota | exame_sem_imagem_e_bloqueado_e_sem_bloqueador_e_penalizado | Não simula alteração concreta de metadados |
| R05 | POP 5.01 §4.2 | Verificar hash da cópia | `imagem` calcula SHA-256; `verificar-hash` confere com recálculo e hash documentado | fluxo_501_d4_hash_documentado_confere; vetores_fips | Hash calculado sobre a representação canônica do modelo |
| R06 | POP 5.01 §4.2 | Cifragem dependente do equipamento → espelhamento com destino sanitizado | `imagem` recusa mídia FDE; `clone` exige `sanitizar` | (unitário de engine — pendente de caso gerado com FDE) | Geração atual não sorteia FDE por hardware |
| R07 | POP 5.01 §4.4 | Recuperação de apagados | `recuperar` integral/parcial/inviável conforme o modelo; setores ruins impedem | fluxo_501_d2_recuperacao_e_composto | Parcial = truncamento do conteúdo |
| R08 | POP 5.01 §4.4 | Expansão de arquivos compostos | Conteúdo de zip só acessível após `expandir` | fluxo_501_d2_recuperacao_e_composto | Um nível de aninhamento |
| R09 | POP 5.01 §4.4 | OCR | `ocr` revela texto de imagens; busca inclui OCR | — | Texto pré-definido no caso |
| R10 | POP 5.01 §4.5 | Senhas por dicionário com dados biográficos | `dicionario` gera candidatos; `decifrar` testa | fluxo_501_d3_conteiner_cifrado_por_dicionario | Sem força bruta temporal |
| R11 | POP 5.01 (pontos críticos) | Fuso horário/horário de verão | Partição em UTC no D3+; `fs meta` alerta | avaliação c7 (fs-meta) | Sem horário de verão histórico |
| R12 | POP 5.01 (pontos críticos) | Nuvem só com autorização judicial | D4 sorteia autorização; credenciais encontradas no disco | — | Acesso à nuvem ainda não exposto no terminal (pendente) |
| R13 | POP 5.02 | Isolar comunicações | `isolar` (Faraday/alumínio/sala/jammer); sem isolamento o comando remoto apaga o aparelho | aparelho_ligado_nao_isolado_e_apagado_remotamente | Momento do comando remoto sorteado |
| R14 | POP 5.02 / 5.05 | Remover SIM registrando slot, operadora, ICCID | `sim remover` registra na auditoria e no termo | fluxo_505_apreensao_gera_termo_nao_laudo | — |
| R15 | POP 5.02 | AFU: manter isolado e carregando | `carregar`; estado AFU habilita extração de sistema de arquivos | fluxo_502_isolado_extrai_e_encontra | Bateria não se esgota ainda (pendente) |
| R16 | POP 5.02 | Extração mais completa possível conforme criptografia/estado | `extrair` escolhe física > FS > lógica conforme suporte e estado | fluxo_502_isolado_extrai_e_encontra | Matriz de suporte simplificada |
| R17 | POP 5.02 | Risco de restauração após tentativas de senha | D4: 10 tentativas erradas apagam o aparelho | tentativas_de_senha_levam_a_apagamento_no_d4 | Limite fixo |
| R18 | POP 5.02 | Segundo espaço / cofres | D4 pode conter cofre cifrado na partição do aparelho | toda_fonte_da_palavra_chave_existe_e_contem_a_expressao | — |
| R19 | POP 5.03 §4.3 | Ordem de volatilidade; dump de memória | `memoria` só com equipamento ligado; artefatos de RAM | fluxo_503_coleta_volatil_correta | Conteúdo da RAM é resumido |
| R20 | POP 5.03 §4.3 | Partição cifrada montada → copiar antes de desligar | `volume copiar`; desligar antes perde o acesso | desligar_com_volume_montado_perde_evidencia | — |
| R21 | POP 5.03 §4.2 | Isolamento digital | `rede isolar`; sem ele, acesso remoto apaga a RAM (D3+) | acesso_remoto_apaga_ram_se_nao_isolado | — |
| R22 | POP 5.03 §4.4 | Não iniciar SO de equipamento desligado; mídia em uso não é adquirida ligada | `imagem` recusa mídia de equipamento ligado | midia_em_uso_nao_pode_ser_imageada_ligada | Boot forense/BIOS não modelados |
| R23 | POP 5.03 §4.4 | Mídias camufladas/ocultas | `inspecionar` revela itens; `componente remover` revela slots ocultos | fluxo_503_coleta_volatil_correta | — |
| R24 | POP 5.03 §4.5 | Embalagem antiestática, lacres e hashes no termo | `embalar`, `lacre aplicar`; hashes no anexo | checklist 5.03 | — |
| R25 | POP 5.04 | Conexão desvinculada da instituição | `conexao`; host monitorado + conexão não anônima alerta o investigado e derruba páginas | diligencia_sem_anonimato_em_host_monitorado_alerta_e_derruba_pagina | — |
| R26 | POP 5.04 | WHOIS/DNS/traceroute coerentes | Consultas sobre o mesmo modelo de rede | fluxo_504_rede_emulada_coerente; horarios_coerentes | Rota simplificada (4 saltos) |
| R27 | POP 5.04 | Cabeçalho completo, Received em ordem inversa, X-Originating-IP, SPF/DKIM | `email cabecalho` lista saltos da origem ao destino | fluxo_504_rede_emulada_coerente | ARC não modelado |
| R28 | POP 5.04 | Preservar página com data/hora/IP; sítios de histórico | `preservar` gera artefato com hash; `arquivo-web` | fluxo_504_rede_emulada_coerente | — |
| R29 | POP 5.04 | Atribuição depende do provedor; CGNAT exige porta | `rede logs` mostra porta/assinante e alerta que conexão ≠ pessoa | fluxo_504_rede_emulada_coerente; avaliação c8 | — |
| R30 | POP 5.05 (p.63) | Estrutura de laudo não se aplica; documento é o termo | Módulo 5.05 gera termo de apreensão, sem palavra-chave | fluxo_505_apreensao_gera_termo_nao_laudo | Campos do termo derivados dos itens do POP |
| R31 | Especificação + prompt | Reprodutibilidade por semente e versão | ID do caso codifica módulo, dificuldade e semente | mesma_semente_reproduz_o_caso; id_do_caso_ida_e_volta | — |
| R32 | Prompt §11 | Coerência relatório × auditoria | check_coherence aponta procedimentos alegados e não realizados e conclusões categóricas | relatorio_incoerente_com_historico_e_detectado | Heurística por palavras-chave |
| R33 | Prompt §8 | Isolamento de rede e sem execução de comandos do SO | Motor sem bibliotecas de rede/processo | motor_nao_usa_rede_nem_processos; terminal_nao_executa_comandos_do_sistema | Verificação estática do código-fonte |
| R34 | Prompt §10 | Palavra-chave não compensa destruição | Perda de evidência limita a nota a 60% | palavra_chave_por_acaso_nao_compensa_destruicao | — |
| R35 | POP 5.02 / 5.05 | AFU: manter isolado E carregando | Bateria descarrega com o tempo (mais rápido isolado procurando sinal); ao zerar, desliga e perde o estado AFU | bateria_esgota_em_afu_sem_alimentacao_e_perde_estado; bateria_externa_preserva_afu | Taxas de descarga ilustrativas |
| R36 | POP 5.02 | IMEI físico × lógico | `imei verificar` compara etiqueta com *#06#; divergência (D4) deve constar no laudo | imei_fisico_x_logico_detecta_divergencia | — |
| R37 | POP 5.02 (eSIM) / 5.05 | Ligar o aparelho somente dentro do isolamento; desligar só sem interesse em AFU | `energia ligar|desligar` em móveis: ligar fora do isolamento é falha; desligar em AFU antes da extração descarta o estado | ligar_celular_fora_do_isolamento_e_falha_e_desligar_em_afu_descarta_estado | — |
| R38 | POP 5.01 §4.2 / 5.03 | Exame sobre a cópia, somente leitura; sem montar o original | Linux da estação: imagens montadas `ro`; montar o dispositivo original é registrado e, sem bloqueador, contamina | exame_501_completo_pelo_terminal_linux; montar_original_e_registrado_e_sem_bloqueador_contamina | Shell é um subconjunto do bash |
| R39 | POP 5.02 / 5.05 | Navegação manual na tela só como complemento documentado; na apreensão, não examinar o conteúdo | Tela do aparelho (FoneOS): cada app aberto vira `tela <item> <app>` na auditoria; navegar antes da extração altera o estado (mensagens lidas) e é falta; em 5.05, abrir conteúdo é falta, configurações/alarmes/IMEI não; `tela <item> foto` documenta | navegar_antes_da_extracao_e_falta_e_marca_mensagens_como_lidas; navegar_depois_da_extracao_e_complemento_documentado; apreensao_permite_configuracoes_mas_nao_conteudo | Conteúdo exibido deriva do modelo do caso; sem teclado de desbloqueio na tela |
| R40 | POP 5.02 / 5.05 | Tela bloqueada não expõe conteúdo; IMEI pelo código de serviço | BFU: sem notificações; AFU: contagem sem conteúdo; discador de emergência com *#06# (exige ligado) | bloqueado_nao_expoe_conteudo_pela_tela; imei_pela_tela_usa_o_codigo_de_servico; desligado_mostra_tela_apagada | Modelo didático genérico, não reproduz um sistema real |
