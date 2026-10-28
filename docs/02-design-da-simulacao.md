# Design da simulação (método ISAGA / Duke)

Referências de método: Peters & van de Westelaken (2014), *Simulation games – a concise introduction to the design process* (ISAGA); Q99 Studio, *How to build a simulation game*; Summer Engine, *How to make a simulation game with AI*. Referência de conteúdo: POP Perícia Criminal 2024 — Informática Forense, vol. 5 (SENASP/MJSP).

## 1. Especificação (fase 1)

| Pergunta do checklist ISAGA | Decisão |
|---|---|
| Por que um jogo de simulação? | Procedimentos periciais são sequenciais e irreversíveis; errar num caso real destrói prova. A simulação permite errar, ver a consequência e refletir. |
| Objetivo de aprendizagem | Executar os POPs 5.01–5.05 na ordem correta, preservar a evidência, interpretar resultados com seus limites e documentar de forma coerente com o que foi feito. |
| Público | Peritos em formação, estudantes de computação forense. Uso individual. |
| Tom | Intelectual (decisão e informação), não reflexo. Sem cronômetro real; há **tempo simulado** que avança a cada ação. |
| Abstração | Média: equipamentos e dados são modelos; procedimentos são fiéis ao POP. Todo resultado simulado é coerente com o estado do caso. |
| Avaliação | Indicadores explícitos, explicados ao final (sem pontuação misteriosa). |

## 2. Análise do sistema (fase 2)

Atores e elementos do sistema de referência (perícia de informática):

* **Atores:** perito (papel jogado), autoridade requisitante (papel simulado — histórico, quesitos, autorização legal), investigado (papel simulado — pode reagir: apagamento remoto, retirada de página), provedor (papel simulado — registros de conexão/CGNAT), proprietário do aparelho (fornece senhas no termo).
* **Objetos:** itens físicos (computadores, celulares, mídias), mídias e partições, arquivos (ativos, apagados, compostos, cifrados), memória volátil, rede (domínios, DNS, hosts, páginas, e-mail, logs), documentos (termo, laudo).
* **Relações críticas:** lacre ↔ documentação; original ↔ cópia (hash); equipamento ligado ↔ dados voláteis; conectividade ↔ risco de comando remoto; IP ↔ assinante ↔ pessoa (não equivalentes); relatório ↔ auditoria.

## 3. Matriz componentes do sistema × elementos de jogo (fase 3)

| Componente \ Elemento | Cenário | Papéis | Regras | Eventos | Dados | Indicadores |
|---|---|---|---|---|---|---|
| Recebimento e lacre | Material lacrado com documentação | Perito | Conferir antes de deslacrar | Divergência de lacre (D≥2) | Nº do lacre físico × documental | Conformidade; custódia |
| Preservação/aquisição | Bancada com bloqueador | Perito | Exame só sobre cópia; bloqueador | — | Hash SHA-256 | Preservação; integridade |
| Análise de arquivos | Imagem montada no PeritOS | Perito | Apagado → recuperar; composto → expandir; cifrado → decifrar | Setores ruins, sobrescrita parcial | Metadados, fuso, OCR | Uso de ferramentas; interpretação |
| Dados voláteis | Local com PC ligado | Perito; investigado (remoto) | Coletar memória/volume antes de desligar | Apagamento remoto se conectado (D≥3) | Artefatos de RAM | Preservação |
| Dispositivo móvel | Aparelho ligado/AFU | Perito; proprietário | Isolar, SIM, alimentação, extração possível | Comando remoto; limite de tentativas | IMEI, ICCID, suporte de extração | Preservação; conformidade |
| Rede | Domínio, página, e-mail, ISP | Perito; investigado; provedor | Conexão desvinculada; preservar; correlacionar | Retirada da página se alertado | DNS, WHOIS, Received, CGNAT | Correlação; limitações |
| Documento | Editor de laudo/termo | Perito | Estrutura por POP; 5.05 = termo | — | Auditoria encadeada | Qualidade documental; coerência |

## 4. Formato do jogo

* **Macro-ciclo:** menu → briefing (caso, quesitos, autorização) → rodadas de ação → documento → finalização → avaliação/*debriefing* → histórico.
* **Micro-ciclo (a cada ação):** escolher ação (botão na cena 3D ou comando) → motor aplica a regra do POP → consequência e avanço do tempo simulado → registro encadeado → retorno textual no terminal.
* **Progressão por dificuldade:** 1 Guiado (mentor com checklist e “por quê”), 2 Assistido (próximo passo sob demanda), 3 Autônomo (só contagem de etapas), 4 Avançado (sem orientação; eventos adversos e evidências distribuídas).
* **Eventos:** planejados (apagamento remoto em instante sorteado pela semente), condicionais (retirada da página quando o investigado é alertado), de consequência (restauração após tentativas de senha).

## 5. Sistema de contabilização

Oito critérios, totalizando 100 pontos (90 na apreensão, que não tem expressão-alvo): conformidade (20), preservação/integridade (20), registro/custódia (10), ferramentas/interpretação (10), correlação (10), fato × inferência e limitações (10), qualidade documental (10), expressão fundamentada (10). Cada perda de ponto vem com a frase que a explica. **Trava:** perda/destruição de evidência limita a nota a 60% — a expressão encontrada não compensa.

## 6. *Debriefing*

A tela de avaliação mostra, por critério, as decisões e consequências; lista as fontes da expressão no caso (revisão); o laudo exportado inclui “Procedimentos não realizados”. O histórico permite rever o mesmo caso pelo ID e tentar de novo.

## 7. Testes do design (regra dos dez, adaptada)

*Talk-through* → matriz acima; *walk-through* automatizado → testes `fluxo_*` que jogam cada módulo; *test runs* → roteiros da interface sob Xvfb com capturas; ajuste de números (ex.: atraso do comando remoto, tentativas de senha) feito observando os testes de consequência.
