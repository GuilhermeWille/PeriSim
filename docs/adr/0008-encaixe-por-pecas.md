# ADR 0008 — Encaixe por peças

**Situação:** aceita (v0.11.0, 29/10/2026)

## Contexto

Na v0.10 a ferramenta certa na mão e um **E** no objeto bastavam: mirava-se o gabinete inteiro
e "soltava-se um parafuso", sem saber onde ele estava. O objetivo do simulador é o manuseio
físico realista; faltava ter **onde** encaixar a ferramenta e o cabo.

## Decisão

* Cada equipamento tem **peças** com posição física: parafusos de dedo da tampa (traseira do
  gabinete), tampa lateral, lacre, botão de energia, porta USB frontal, porta de rede traseira,
  conectores SATA de dados e de energia de cada disco, parafusos do suporte (dois de cada lado),
  a própria mídia; no notebook, desligado e inspecionado, ele fica fechado e de cabeça para
  baixo, com os quatro parafusos e a tampa de acesso; no celular, porta de dados/carga, gaveta do
  chip e câmera frontal; no bloqueador, portas SATA, M.2, USB de saída (traseira), leitor SD e
  USB de entrada (lateral) e a chave de proteção; na estação, a porta USB frontal do gabinete.
* A geometria fica num só lugar (`crates/app/src/parts.rs`). O mundo 3D desenha o que o modelo
  não traz e cria um alvo de mira `P:<dono>:<peça>` para cada uma; as etapas
  (`steps.rs`) apontam para a peça (`Alvo::Parte`) **quando ela existe no estado atual** —
  senão continuam no objeto inteiro. Assim nenhuma etapa pendente fica sem alvo.
* Peças repetidas levam sufixo numérico (`parafuso.0`, `sata0.parafuso.3`); a etapa "soltar os
  parafusos" vale para qualquer um que ainda esteja lá. Parafuso solto deixa de ser desenhado;
  cabo desconectado fica com a ponta solta, caída.
* Mira: a peça vence o objeto a que pertence quando está até 30 cm atrás da superfície dele
  (enxerga-se pela tampa aberta), mas não as do outro lado — para soltar os parafusos da
  traseira é preciso dar a volta. O gabinete no piso do escritório saiu de baixo da mesa para
  o lado dela, para dar a volta.
* Cabos ligados são desenhados de porta a porta (tubo fino com barriga; até outra mesa,
  descem ao piso e sobem).
* Fotografar, inspecionar, isolar, embalar, lacrar e etiquetar continuam no objeto inteiro.

## Consequências

* O teste `nenhum_caso_trava_na_ordem` (80 casos) continua passando: o alvo é calculado pela
  mesma função que desenha.
* Dicas e avisos dizem a peça ("— em: parafusos da tampa de ITEM-01").
* Peças muito pequenas ganham caixa de mira um pouco maior que o desenho.
