# ADR 0005 — Modo passo a passo: ferramenta na mão, etapas físicas e vestígios

**Data:** 28/10/2026 · **Situação:** aceita

## Contexto

As ações físicas eram comandos únicos ("abrir", "midia remover", "bloqueio"): o resultado
aparecia sem que o jogador precisasse escolher equipamento nem seguir a ordem real. Para um
simulador de treinamento, o valor está justamente nessas escolhas.

## Decisão

* Novo módulo do motor (`equip.rs`) com o equipamento do perito: uma ferramenta na mão,
  luvas (limpas / usadas em material biológico), pulseira antiestática, parafusos retirados,
  cabos desconectados, mídias retiradas e a cadeia mídia → bloqueador → estação.
* Um verificador de requisitos roda antes de cada comando: sem a ferramenta certa ou sem a
  etapa anterior, a ação não acontece e nada entra na auditoria. Luvas e pulseira são
  "vestidas": sem elas a ação ocorre e vira falta, porque o erro é possível na vida real.
* Novos comandos auditados: `pegar`, `guardar`, `luvas`, `pulseira`, `desparafusar`, `cabo`,
  `conectar`, `marcar`, `comunicar`, `equipamento`.
* Vestígios não digitais entram no caso (`vestigios`), gerados por semente. O perito sinaliza,
  fotografa e comunica; não coleta. Itens com vestígio biológico contaminam as luvas.
* Versionamento: tudo isso vale para casos do gerador **1.2.0**. Casos 1.0.0/1.1.0 seguem as
  regras clássicas — salvamentos antigos são reproduzidos sem divergência (ADR 0003).

## Consequências

* Fluxos mais longos e mais próximos da realidade; a interface mostra só a próxima etapa
  possível e o ícone da ferramenta que falta, para não virar tentativa e erro.
* O conteúdo-base do caso não muda entre 1.1.0 e 1.2.0 (mesmas mídias, pessoas e expressão).
* Testes: `tests/passo_a_passo.rs` cobre o fluxo completo de 5.01, bloqueios por ferramenta e
  por ordem, ligação direta sem bloqueador (falta), contaminação cruzada, vestígios, cabo do
  celular por sistema e determinismo do replay.
