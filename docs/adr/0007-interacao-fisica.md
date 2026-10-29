# ADR 0007 — Interação física: bancada, etapas em ordem e exame de perto

**Data:** 29/10/2026 · **Situação:** aceita

## Contexto

As etapas físicas eram botões num painel lateral: bastava clicar em ordem, e a ferramenta só
precisava estar "na mão" no motor. A maleta guardava tudo num menu, enquanto a câmera ficava
solta na mesa — incoerente. O objetivo é que o perito faça cada etapa como na vida real.

## Decisão

* **Sem painel de botões na cena 3D.** As etapas de cada objeto vêm da mesma lista do motor
  (`steps.rs`), com a ferramenta exigida (`equip::requirements`) e o alvo (objeto, mídia,
  bloqueador, estação, vestígio). E no alvo executa a próxima etapa que a ferramenta da mão faz
  ali; uma etapa obrigatória anterior pendente impede pular a ordem. Etapas opcionais sem
  ferramenta (modo avião, alarmes) só acontecem quando marcadas como objetivo.
* **Bancada no lugar da maleta.** Cada ferramenta é um objeto da cena; E pega, G larga onde se
  mira. A arrumação é do jogador e fica em `saves/bancada.json`, por cenário.
* **Indicadores e cabos ponta a ponta.** Com uma ferramenta na mão, cada alvo onde ela serve
  ganha um marcador. Etapas de dois alvos (mídia ↔ bloqueador, bloqueador ↔ estação) pedem as
  duas pontas; o comando do motor só roda ao conectar a segunda.
* **Exame de perto (F)**: a câmera orbita o objeto real da cena e mostra o que é legível nele.
* **Objetivo marcado** no tablet, exibido discretamente na HUD, avançando sozinho.
* O motor não mudou: cada ação continua sendo um comando auditado, e o terminal continua aceitando
  tudo. A ordem é regra da interface; a avaliação continua no checklist do motor.

## Consequências

* Um teste conclui todas as etapas obrigatórias de 80 casos (5 módulos × 4 dificuldades × 4
  sementes) só com o que a regra libera — garante que nenhum caso trava na ordem. Ele mostrou três
  armadilhas, corrigidas: extração impossível em BFU, disco cifrado por hardware (sem imagem fora
  do equipamento) e uma etapa opcional que o motor recusa (modo avião com tela bloqueada).
* No celular, isolar e ligar à bateria externa vêm logo depois da fotografia, para que a ordem
  obrigatória não atrase o isolamento (apagamento remoto) nem deixe a bateria acabar.
* O erro proposital "ligar a mídia direto na estação" saiu da cena (continua possível no terminal).
