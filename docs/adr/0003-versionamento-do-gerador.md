# ADR‑0003 — Versões do gerador por extensão

Status: aceita · Data: 28/10/2026

## Contexto

A persistência é por replay (ADR‑0002): o salvamento guarda o ID do caso, a versão do gerador e os comandos. Até a v0.3.1, mudar o gerador tornava todos os salvamentos irreproduzíveis (o carregamento era recusado), porque a semente do caso dependia do texto da versão. A v0.4 precisava acrescentar cenários (cifragem por hardware, arquivos danificados, cadeia ARC) sem perder os casos já salvos nem mudar o conteúdo dos IDs citados na documentação.

## Decisão

* O conteúdo-base é gerado com um rótulo de semente fixo (`1.0.0`), igual para todas as versões.
* Cada versão nova só **acrescenta** extensões (`casegen_ext.rs`), com sementes próprias, quando o caso é gerado nessa versão ou acima.
* `generate_v(módulo, dificuldade, semente, versão)` reproduz qualquer versão listada em `SUPPORTED_GEN_VERSIONS` (hoje `1.0.0` e `1.1.0`); o carregamento usa a versão gravada no salvamento.
* Hashes registrados "em documento anterior" são recalculados depois das extensões, porque descrevem a mídia como ela é.

## Consequências

* Salvamentos da v0.3 carregam e reproduzem o mesmo caso; um teste garante a igualdade da versão 1.0.0 e a preservação do conteúdo-base.
* Mudanças nas **regras** do motor (não no gerador) ainda podem fazer um replay antigo divergir. Por isso o salvamento passou a gravar a trilha da auditoria, e o carregamento avisa a partir de qual entrada a reprodução divergiu, em vez de recusar.
* Extensões não podem remover nem alterar o conteúdo-base; quando isso for necessário, será preciso uma nova linha de base e manter a antiga em `SUPPORTED_GEN_VERSIONS`.
