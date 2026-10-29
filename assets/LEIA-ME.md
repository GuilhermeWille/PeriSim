# Recursos visuais externos (modelos 3D e texturas)

Esta pasta guarda os modelos 3D do Perícia Sim e recebe texturas/modelos fornecidos. Nada é obrigatório: sem um arquivo aqui, o simulador usa a geometria procedural correspondente.

## Modelos incluídos (gerados no Blender 5.2)

`models/` traz 22 modelos OBJ/MTL gerados pelo script `tools/blender/gerar_modelos.py`; a fonte editável é `blender/modelos.blend`. Cerca de 11 600 triângulos no total.

| Arquivo | Onde aparece | Triângulos |
|---|---|---|
| `cadeira_escritorio` | cadeiras do laboratório e do escritório | 1 348 |
| `mesa_escritorio` | mesa do investigado (POP 5.03) | 488 |
| `mesa_estacao` | mesa da estação pericial | 488 |
| `monitor` | monitores (a área `tela` recebe a imagem da sessão Linux) | 298 |
| `gabinete_pc` | gabinete da estação e computadores apreendidos fechados | 432 |
| `notebook` | notebooks apreendidos (a `tela` mostra o estado) | 322 |
| `celular` | celulares apreendidos (a `tela` mostra o estado) | 374 |
| `bancada_lab` | bancada antiestática do laboratório | 952 |
| `armario_aco` | armário de evidências | 548 |
| `estante_aco` | estante de caixas de evidência | 560 |
| `estante_livros` | estante com livros do escritório | 2 284 |
| `gaveteiro` | gaveteiro com a gaveta aberta | 444 |
| `planta` | planta do escritório | 564 |
| `camera_foto` | câmera fotográfica do perito | 720 |
| `bloqueador_escrita` | bloqueador de escrita (os LEDs de estado são do jogo) | 488 |
| `kit_embalagem` | Faraday bag, saco antiestático, fita e lacres | 316 |
| `luminaria_bancada` | luminária articulada da bancada | 254 |
| `teclado` | teclados da estação e dos computadores ligados no local | 80 |
| `mouse` | mouse da estação | 236 |
| `hd_35` | HDs apreendidos e HDs dentro de gabinetes abertos | 190 |
| `ssd_25` | SSDs apreendidos | 68 |
| `pendrive` | pendrives apreendidos | 188 |

Para regenerar depois de editar o script: `python3 tools/blender/gerar_modelos.py assets` (com o módulo `bpy`) ou `blender --background --python tools/blender/gerar_modelos.py -- assets`. Prévias renderizadas: `python3 tools/blender/renderizar_previas.py assets <pasta>`.

Elementos que mudam com o caso (lacres, LEDs, telas, tampa aberta com componentes, mídias, ferramentas) continuam desenhados pelo jogo por cima dos modelos ou no lugar deles quando o estado exige (ex.: gabinete aberto).

Convenções: metros; origem no centro da base; frente para +Z no jogo (+Y no Blender); o prefixo do nome do material escolhe a textura (`madeira`, `metal`, `plastico`, `tecido`, `manta`, `grade`, `teclado`, `etiqueta`, `ceramica`, `folhagem`, `tela`…); `Kd` tinge e `Ns` define o brilho.

## Regras

1. **Originais intocados.** Guarde os arquivos recebidos como vieram (por exemplo, em `originais/`). O programa só lê; relatórios são gravados na pasta de dados do usuário (`assets_relatorio.txt`), nunca aqui.
2. **Inventário antes de usar.** Rode
   `cargo run --release -p pericia_core --example asset_inventory -- assets/originais docs/inventario-assets.md`
   para listar formato, tamanho, dimensões, contagens de malha, referências ausentes, licenças e o SHA-256 de cada arquivo (para provar depois que nada foi alterado).
3. **Licença.** Sem arquivo de licença, não presumir uso livre: confirmar com quem forneceu.
4. **Integração gradual.** Um elemento por vez, comparando capturas antes/depois e medindo quadro e memória.

## Texturas (já funcionam)

Coloque em `assets/textures/<nome>.png` (ou `.tga`). Cada arquivo substitui a textura procedural de mesmo nome. Texturas repetidas (todas, exceto `sombra_contato`) precisam ter lados em **potência de 2** (256, 512, 1024, 2048) — exigência do OpenGL 2.1/GLES 2 usado para rodar sem GPU dedicada. Limite: 2048 px por lado. Arquivo inválido é recusado com o motivo e a textura procedural é mantida.

| Nome do arquivo | Onde aparece |
|---|---|
| `madeira` | mesas, estante, gaveteiro |
| `piso_madeira` | piso do escritório |
| `piso_vinilico` | piso do laboratório |
| `parede` | paredes |
| `forro` | teto |
| `metal` | armário, pés, bancada |
| `plastico` | gabinetes, monitores, periféricos |
| `tecido` | cadeiras |
| `manta_antiestatica` | tampo da bancada |
| `grade_ventilacao` | grades dos gabinetes |
| `teclado` | teclas |
| `etiqueta` | etiquetas e lacres |
| `tapete` | tapete do escritório |
| `folhagem` | planta |
| `papelao` | caixas |
| `tela` | monitor sem sessão |
| `concreto` | vaso |
| `vidro` | vidros |
| `vista_janela` | paisagem vista pela janela |
| `sombra_contato` | sombra sob objetos (sem repetição) |

JPEG ainda não é decodificado: converta para PNG (ou habilite o decodificador, decisão a tomar ao avaliar os arquivos).

## Modelos

Arquivos **OBJ/MTL** em `assets/models/` são lidos e validados na inicialização; o relatório vai para `assets_relatorio.txt` na pasta de dados. Um modelo com o mesmo nome de um da tabela acima o substitui diretamente; modelos com outros nomes ficam carregados, mas só aparecem quando o código da cena os usar.

glTF/GLB, FBX, COLLADA e .blend: o inventário os descreve, mas carregar exige conversão para OBJ ou uma dependência nova (leitor glTF). Essa escolha será feita quando os arquivos chegarem, pela alternativa menos invasiva.

A cena usa **metros**, Y para cima. A malha do macroquad usa índices de 16 bits: modelos com mais de 65 535 vértices precisam ser divididos.
