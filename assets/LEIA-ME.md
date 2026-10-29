# Recursos visuais externos (modelos 3D e texturas)

Esta pasta recebe modelos e texturas fornecidos para o Perícia Sim. **Ela está vazia de propósito**: sem arquivos aqui, o simulador usa a geometria e as texturas procedurais de sempre. Nada é obrigatório.

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

Hoje: arquivos **OBJ/MTL** em `assets/models/` são lidos e **validados** na inicialização (triangulação, vértices, materiais, caixa envolvente, texturas referenciadas); o resultado vai para `assets_relatorio.txt`. A colocação de um modelo na cena é feita depois de avaliar escala e aparência, um elemento por vez.

glTF/GLB, FBX, COLLADA e .blend: o inventário os descreve, mas carregar exige conversão para OBJ ou uma dependência nova (leitor glTF). Essa escolha será feita quando os arquivos chegarem, pela alternativa menos invasiva.

A cena usa **metros**, Y para cima. A malha do macroquad usa índices de 16 bits: modelos com mais de 65 535 vértices precisam ser divididos.
