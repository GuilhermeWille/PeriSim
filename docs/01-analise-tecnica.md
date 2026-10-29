# Análise técnica inicial — Simulador de Informática Forense

Data: 27/10/2026 · Versão do documento: 1.0

## 1. Materiais efetivamente lidos

| Material | Situação |
|---|---|
| `pericia_criminal_2024_informatica_forense-vol-5-.pdf` (69 p., SENASP/MJSP 2024) | Lido integralmente (POPs 5.01 a 5.05, glossários e referências). |
| `escopo_simulador_informatica_forense_ABNT.pdf` (19 p.) | Lido integralmente (escopo, módulos, rede emulada, critérios de aceitação). |
| Prompt mestre do solicitante | Lido; define requisitos adicionais e regras de trabalho. |
| q99studio.com — *How to build a simulation game* | Lido: conceito → sistemas inter-relacionados → *core loop* → protótipo → feedback → iteração. |
| summerengine.com — *How to make a simulation game with AI* | Lido: um laço mínimo por vez, validar antes de adicionar, ajuste de números é o “jogo real”. |
| ISAGA — Peters & van de Westelaken (2014), *Simulation games: design process* | Lido: 4 fases/10 passos (especificação, análise do sistema, desenho do jogo, construção/teste), matriz componentes × elementos, papéis (jogados, pseudo, simulados), eventos (planejados, aleatórios, *ad hoc*), indicadores, sistema de contabilização e *debriefing*. |

## 2. Requisitos extraídos por POP (resumo; detalhes na matriz de rastreabilidade)

**POP 5.01 — Mídias de armazenamento (p. 13–21).** Atenção a vestígios físicos; conferir descrição e lacre *antes* do deslacre; fotografar lacrado; deslacrar e reconferir; remover a mídia do equipamento quando viável e registrar a interface/porta; exame sobre a cópia (original só se a cópia for inviável); bloqueador de escrita (hardware ou software) ou equipamento forense; preferência por “mídia para arquivo‑imagem”; verificação de *hash*; criptografia dependente do equipamento → espelhamento mídia‑para‑mídia com sanitização prévia do destino; processamento (recuperação, indexação, categorização, *hash*, detecção de criptografia, expansão de compostos, OCR); análise (extração direta / elucidação); tratamento de criptografia (recuperação direta, *rainbow tables*, força bruta, dicionário com regras); laudo (material identificado, original vs. cópia, recuperação, softwares, alterações, *hash* da lista de *hashes* do anexo, novo lacre). Pontos críticos: RAID/JBOD, ordem de boot na BIOS, credenciais de nuvem só com autorização judicial, fuso horário/horário de verão.

**POP 5.02 — Dispositivos móveis (p. 25–33).** Isolamento RF (gaiola/jammer); remover SIM registrando o *slot*; microSD removido só para identificação/duplicação e reinserido; AFU: manter isolado e carregando; bateria; reparo quando viável; eSIM → ligar dentro da gaiola; modo avião e verificação de todas as conexões; desbloqueio por ferramenta ou dicionário, com risco de restauração de fábrica; desabilitar alarmes; extração mais completa possível (física × sistema de arquivos conforme o tipo de criptografia); em AFU, verificar integridade da extração antes de desligar; PIN bloqueia após 3 e PUK após 10 tentativas; número da linha e correio de voz vêm da operadora; IMEI físico × lógico; segundo espaço/cofres; laudo com IMEIs, ICCID, operadora.

**POP 5.03 — Local de informática (p. 37–45).** Levantamento prévio; restrições legais/mandado; reconhecer equipamentos; isolamento físico e digital; fixação fotográfica; equipamento ligado: ordem de volatilidade, *dump* de memória quando há interesse (ex.: suspeita de criptografia), programas em execução, partições cifradas montadas → copiar conteúdo; apreensão → cortar energia; partição cifrada → apreender equipamento completo; servidor → auxílio do responsável de TI (verificando se não é suspeito); equipamento desligado: não iniciar o SO instalado (bloqueador ou boot forense + ordem de boot na BIOS); mídias camufladas/ocultas (SSD sob dissipador/placa de vídeo); *hashes* e lacres no termo de apreensão; embalagem antiestática.

**POP 5.04 — Local de internet (p. 49–55).** Conexão desvinculada da instituição; autorização legal; IP/domínio: WHOIS, traceroute, DNS reverso, geolocalização; atribuição depende do provedor (e da porta lógica em CGNAT); e‑mail: cabeçalho completo, campos *Received* em ordem inversa, consistência de horários e IP/domínio, *open relay*, X‑Originating‑IP, DKIM, SPF, ARC; sites: preservar no formato original, registrar data/hora e IP usado, novas tentativas e sítios de histórico; nuvem: preferir quebra judicial; captura direta com credenciais tem risco de bloqueio. Prioridade pela volatilidade; dados de WHOIS podem ser falsos.

**POP 5.05 — Apreensão de móveis (p. 59–65).** Baterias externas carregadas; reconhecer, isolar e fixar; solicitar todos os aparelhos; proprietário não manipula; etiqueta na câmera frontal (nada mais na tela); solicitar senhas (inclusive de segundo espaço/cofres), verificar *smartlock*; anotar senhas no termo e em etiqueta no verso; modo avião e registro se foi possível; desabilitar alarmes; sempre remover SIM, registrando slot, operadora e ICCID; SIM afixado no verso; desligar se não houver interesse AFU; bateria removível retirada; cartões de memória relacionados e mantidos no aparelho; AFU → Faraday bag ou ≥ 3 camadas de alumínio + bateria externa; embalar e lacrar individualmente; aparelho danificado, molhado ou contaminado → cuidados e etiquetas. **Estrutura de laudo: “Não se aplica”** → documento é o termo de apreensão.

## 3. Ambiguidades e riscos

1. O POP 5.03 tem duas seções intituladas “Ações Preliminares” (4.1 e 4.2). Interpretação: 4.1 = ações preparatórias (antes do local), 4.2 = ações no local.
2. A especificação cita a estrutura de laudo “conforme POP 5.03”, mas a mesma estrutura aparece nos POPs 5.01–5.04; o 5.05 diz “Não se aplica”. O simulador aplica a estrutura aos quatro primeiros.
3. Os POPs não definem campos de um **termo de apreensão**; os campos foram derivados dos itens que o POP manda “constar no termo” (5.03 §4.5; 5.05 §4.3) — marcados como *decisão de projeto*.
4. O algoritmo de *hash* não é nomeado (“criptograficamente seguro”). Decisão: SHA‑256.
5. Trechos do POP 5.04 (DKIM/SPF) aparecem com a ordem quebrada na paginação; interpretados pelo sentido.
6. O POP 5.03 menciona bases de *hashes* de exploração sexual infantil. O simulador **não gera nem descreve esse tipo de conteúdo**; usa “base de *hashes* de referência” de documentos sigilosos vazados (fictícios). Limitação deliberada.
7. A especificação recomendava Rust + egui (2D); o solicitante pediu modelos 3D. Ver ADR‑0001.
8. Risco: escopo muito grande. Mitigação: motor determinístico testável e incrementos verificáveis.
9. Risco: fidelidade. Recuperação, extração móvel e memória volátil são **modelos**; cada resultado mostra o rótulo “SIMULADO”.

## 4. Arquitetura (revisada após a escolha de Rust — ver ADR-0001 rev. 2)

```
┌──────────────────────── pericia_app (Rust + macroquad) ────────────────────────┐
│ Área de Hardware 3D                   │ Estação Pericial → "PeritOS" (2D)       │
│  cena procedural, seleção por raio,   │  terminal, evidências, navegador da     │
│  painel de ações físicas por POP      │  rede emulada, custódia, linha do tempo,│
│                                       │  documento, manual POP, mentor, avaliação│
└───────────────▲───────────────────────┴───────────────▲─────────────────────────┘
                │ exec("comando")  →  Output              │
┌───────────────┴──────────── pericia_core (Rust puro, só serde) ─────────────────┐
│ casegen (semente+versão) │ vfs (evidências sob demanda, hash, recuperação)       │
│ engine (regras dos POPs, eventos, tempo simulado) │ tools (terminal, rede emulada)│
│ audit (cadeia SHA-256) │ report + pdf (laudo/termo, Markdown, PDF próprio)       │
│ assess (avaliação explicada, mentor) │ manual (resumos POP, matriz)              │
└──────────────────────────────────────────────────────────────────────────────────┘
Persistência: {ID do caso, versão do gerador, comandos} → estado reconstruído por replay (ADR-0002).
```

## 5. Escolha tecnológica

Rust (motor e interface). Interface com macroquad (OpenGL 2.1/GLES 2, binário ~4 MB). Validada por protótipo antes da consolidação (ADR-0001).

## 6. Estrutura

```
crates/core   motor (sem gráficos, sem rede)     crates/core/tests   testes de integração
crates/app    interface 3D/2D                     crates/core/examples bench, laudo de exemplo, matriz
docs          análise, design, desempenho, ADRs, matriz de rastreabilidade
```

## 7. Primeiro incremento e testes (realizado)

Incremento 1: PRNG com semente, SHA‑256 próprio, auditoria encadeada, gerador de casos versionado.
Testes: vetores oficiais do SHA‑256; mesma semente+versão ⇒ caso idêntico (hash canônico); sementes diferentes ⇒ casos diferentes; palavra‑chave presente em pelo menos uma fonte alcançável; adulteração da auditoria é detectada.
