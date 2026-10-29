# ADR‑0001 — Tecnologia da aplicação

Status: **aceita** · Data: 27/10/2026 · Revisão 2 (substitui a revisão 1, que propunha TypeScript/WebGL)

## Contexto

A especificação preliminar sugeria Rust + egui/eframe (2D). O solicitante pediu depois **modelos 3D de preferência**, duas áreas (hardware físico e estação com software/rede), execução “em qualquer máquina” e, explicitamente, **uma linguagem compilada — não TypeScript/HTML**.

## Opções avaliadas

| Opção | 3D | Requisito de GPU | Binário | Compilação | Observação |
|---|---|---|---|---|---|
| Rust + egui/eframe | sem cena 3D pronta | GL 3 / wgpu | ~5 MB | média | não atende “3D” |
| Rust + Bevy | completo | Vulkan/GL ES 3 / wgpu | 30–80 MB | longa | GPU antiga/integrada pode falhar |
| C++ + raylib | bom | GL 3.3 (ou 2.1 recompilando) | ~2 MB | média | gerência de memória manual, menos testável |
| **Rust + macroquad** | bom (câmera 3D, malhas próprias) | **OpenGL 2.1 / GLES 2** | **~2,6 MB** | **~50 s release** | carrega libGL dinamicamente; roda em software (Mesa) |

## Decisão

Workspace Rust com dois *crates*:

* `pericia_core` — motor puro (sem gráficos e **sem nenhuma API de rede**): casos, evidências, rede emulada, procedimentos POP, auditoria, avaliação, laudo/termo, Markdown e PDF. Dependências: apenas `serde` e `serde_json`.
* `pericia_app` — interface com macroquad: Área de Hardware (cena 3D com modelos *low‑poly* gerados por código e iluminação pré‑calculada nos vértices) e Estação Pericial (desktop 2D com terminal, navegador da rede emulada, evidências, custódia e editor de documentos).

## Validação realizada (protótipo, 27/10/2026)

* `cargo build --release`: 47 s; binário de 2,6 MB.
* Execução sob Xvfb + Mesa **llvmpipe** (renderização por software, sem GPU): 10 quadros em 369 ms incluindo inicialização; texto com acentos (ç, ã, º, §) correto com DejaVu Sans embutida.

## Consequências

* Iluminação calculada na CPU por face (sem *shaders* customizados) → compatível com GL 2.1.
* Interface própria (sem biblioteca de widgets) — mais código, porém controle total e leveza.
* Isolamento de rede é estrutural: o *crate* de motor não depende de nenhuma biblioteca de rede; um teste verifica a árvore de dependências e o código‑fonte.
* Builds Windows/macOS exigem compilação nessas plataformas (ou *cross‑compile*); apenas Linux é validado neste ambiente.
