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
* **Renderização leve:** geometria procedural *low‑poly*, iluminação por face calculada na CPU (sem *shaders*, compatível com GL 2.1), lote de vértices único por quadro.
* **Modo 2D leve** (sem cena 3D) e **economia de energia** configuráveis no menu.

## Limitações das medições

* Uma única máquina, sem GPU; Windows e macOS não foram medidos.
* O hash de mídia é calculado sobre a representação canônica do modelo (arquivos de interesse + amostra do ruído + contagem total), não sobre gigabytes reais — por isso é rápido. Isso é uma simplificação deliberada da simulação.
