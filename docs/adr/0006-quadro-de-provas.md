# ADR 0006 — Quadro de provas com fotos reais da cena

**Data:** 28/10/2026 · **Situação:** aceita

## Contexto

O jogador precisava de um lugar para organizar o que já sabe do caso atual: fotos tiradas na
cena, observações e as ligações entre elas — como o quadro de cortiça de uma investigação.

## Decisão

* Cada `fotografar` bem-sucedido captura a imagem real do quadro renderizado (recorte 4:3,
  400×300, sem as mãos do perito) e grava em `saves/fotos/<caso>/foto_NN.png`, com legenda e
  hora do jogo.
* O quadro (`saves/quadro.json`) guarda, por caso, cartões do tipo foto, bilhete e anotação
  do tablet, com posição livre, e as ligações (barbante). Modos: mover, barbante e remover.
* O quadro é **material de trabalho do jogador**, não evidência: não entra na auditoria nem
  na avaliação, e não altera o replay determinístico do caso.
* Os cartões também são desenhados no modelo `quadro_provas` da parede (fotos como quadros
  texturizados, sem iluminação), para que o quadro exista no cenário.

## Consequências

* Apagar `saves/fotos` ou `quadro.json` não afeta o caso, só o quadro.
* As fotos dependem da janela renderizada; no modo 2D leve não há captura.
