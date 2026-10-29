# Desempenho e consumo de recursos

Medições feitas em 27/10/2026, no ambiente de desenvolvimento descrito abaixo. **Não** são metas: servem de linha de base para comparação em revisões futuras.

## Equipamento de medição

* CPU: Intel Xeon @ 2,80 GHz, **2 vCPUs**; 8 GB de RAM; Linux x86_64 (contêiner).
* **Sem GPU.** Vídeo: Xvfb + Mesa **llvmpipe** (OpenGL por software). É o pior caso realista: em máquinas com GPU integrada, a renderização sai da CPU.
* Build: `cargo build --release` (LTO *thin*, `opt-level=3`, `strip`).

## Aplicação

| Medida | Resultado | Como foi medido |
|---|---|---|
| Tamanho do executável | 3,9 MB (inclui as três fontes DejaVu embutidas, ~1,8 MB) | `ls -la target/release/pericia-sim` |
| Dependências em tempo de execução | libGL/libX11 do sistema (carregadas dinamicamente) | — |
| Tempo de inicialização até o primeiro quadro e encerramento | 0,40 s | roteiro vazio, `getrusage` do processo |
| Memória máxima (RSS) — menu | 185 MB | idem; **inclui o llvmpipe** (JIT e *threads* do Mesa) |
| Memória máxima (RSS) — cena 3D do caso 5.03 D4 | 190 MB | roteiro de 120 quadros |
| Memória máxima (RSS) — PeritOS com texto longo | 213 MB | idem |
| CPU da lógica + montagem da cena por quadro (cena 3D) | média 1,95 ms; p95 2,03 ms | cronômetro interno, excluindo a espera do *vsync* |
| Quadros por segundo, cena 3D, llvmpipe | ≈ 42 fps (131 quadros em 3,1 s) | idem |
| Quadros por segundo, PeritOS com muito texto, llvmpipe | ≈ 28 fps | idem |
| Economia de energia (ocioso) | após 30 quadros sem entrada, ≈ 15–20 redesenhos/s | `sleep(45 ms)` por quadro ocioso |

Observações:

* A maior parte da memória medida pertence ao driver OpenGL por software. O motor em si é pequeno (ver abaixo); com driver de GPU a RSS tende a ser menor, mas isso **não foi medido** aqui.
* Com texto denso, o custo dominante é desenhar os glifos (um quadrilátero por caractere) no rasterizador por software.

## Motor (sem interface)

`cargo run --release -p pericia_core --example bench`

| Medida | Pequeno (D1) | Médio (D3) | Grande (D4) |
|---|---|---|---|
| Geração de caso (média dos 5 módulos, 200 sementes) | 0,069 ms | 0,055 ms | 0,134 ms |
| Tamanho serializado do caso | 8,4 KB | 9,3 KB | 9,8 KB |
| Arquivos representados por caso (reais + de sistema sob demanda) | 50 165 | 74 014 | 74 014 |
| Hash de todas as mídias (caso 5.03) | 0,72 ms | 0,68 ms | 0,67 ms |

| Operação | Tempo |
|---|---|
| Comando do terminal (média, fluxo 5.03 D4 com aquisição e hash) | 0,08 ms |
| Geração do PDF do laudo (≈ 8 KB) | 0,51 ms |
| Replay completo de um salvamento (14 comandos) | 0,96 ms |

## Técnicas de economia aplicadas

* **Carregamento sob demanda:** os ~120 mil arquivos de sistema de uma partição não existem na memória; são sintetizados a partir do índice e da semente quando listados (amostra de até 400 nomes).
* **Persistência mínima:** o salvamento é (ID do caso, versão, lista de comandos). O estado é reconstruído por replay em ~1 ms.
* **Renderização (v0.3):** geometria procedural *low‑poly*, um lote de malha por textura, *shader* GLSL 100 (compatível com GL 2.1/GLES 2) com iluminação por pixel (3 luzes pontuais + sol + ambiente hemisférico + especular + névoa), mapa de sombras 1024² com PCF de 5 amostras, texturas procedurais 256² com *mipmaps* geradas uma vez na inicialização. Medido no escritório em primeira pessoa, 1280×760, Mesa *llvmpipe* (sem GPU): **≈ 10,4 ms de CPU por quadro (p95 11,4 ms)** com sombras. As sombras podem ser desligadas em *Configurações*.
* **v0.4 (28/10/2026), mesma máquina:** escritório em primeira pessoa com sombras, média 10,9 ms de CPU por quadro (p95 12,6 ms) — igual à v0.3 dentro da variação; estação com o editor aberto 2,4 ms (p95 7,8 ms); inicialização até o primeiro quadro 0,38 s; RSS máxima 186 MB (menu) a 248 MB (estação). Motor: geração de caso 0,03–0,14 ms, comando 0,06 ms, PDF 0,54 ms, replay de 14 comandos 0,80 ms. O editor passou a quebrar linhas com custo linear (largura por caractere em cache) para suportar o cursor sem degradar o quadro.
* **v0.6 (28/10/2026), mesma máquina, com os 23 modelos 3D e o tablet em mãos:** escritório 7,5–7,9 ms de CPU por quadro (p95 7,7–8,4 ms), laboratório 7,3–7,4 ms (p95 7,7–8,7 ms), cena com o tablet aberto 8,4–9,0 ms (p95 8,9–9,9 ms). O tablet em mãos vai num lote à parte, fora do passe de sombras e da assinatura do mapa de sombras, para que mover a câmera não force o refazimento do mapa. Layout do tablet conferido em 1024×600, 1280×760 e 1920×1080.
* **v0.7 (28/10/2026), com som:** escritório 7,7 ms e laboratório 8,0 ms de CPU por quadro (p95 8,1 / 7,9 ms) com o mixer ativo — dentro da variação da v0.6. A mixagem roda numa thread própria, em blocos de 1024 quadros (≈ 23 ms); 50 sons ocupam ≈ 17 MB de RAM em ponto flutuante.
* **v0.8 (28/10/2026), com 68 modelos, mãos em primeira pessoa, decoração e música:** escritório 8,6–8,9 ms, laboratório com ferramenta na mão 9,4–9,7 ms de CPU por quadro (p95 8,7–9,3 ms) — cerca de 1 ms a mais que a v0.7, pelos modelos novos. As mãos vão no lote sem sombra (como o tablet da v0.6); decalques de vestígio são desenhados por último, sem entrar no mapa de sombras.
* **Modo 2D leve** (sem cena 3D) e **economia de energia** configuráveis no menu.

## Limitações das medições

* Uma única máquina, sem GPU; Windows e macOS não foram medidos.
* O hash de mídia é calculado sobre a representação canônica do modelo (arquivos de interesse + amostra do ruído + contagem total), não sobre gigabytes reais — por isso é rápido. Isso é uma simplificação deliberada da simulação.
