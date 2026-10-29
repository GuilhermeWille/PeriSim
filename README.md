# Perícia Sim — Simulador de Informática Forense

Simulador de treinamento em perícia de informática, no estilo “hacking simulator”, baseado nos **POPs de Perícia Criminal 2024 — Informática Forense, vol. 5 (SENASP/MJSP)**. Escrito em **Rust**, em **primeira pessoa**, com iluminação por pixel, sombras em tempo real e texturas geradas proceduralmente — roda inclusive sem placa de vídeo dedicada.

> Todo o conteúdo dos casos é **fictício**. A “internet” do jogo é **emulada** dentro do programa: nenhuma ferramenta investigativa acessa a rede real, e o terminal **não** executa comandos do sistema operacional.

## O que é

Duas áreas, como pedido:

* **Ambiente em primeira pessoa (3D)** — laboratório pericial ou local de busca (escritório), percorridos a pé, sem personagem visível. Bancada antiestática, gabinetes, notebook, celular, HDs, pendrives, câmera, bloqueador de escrita, Faraday bag, lacres, armário, estante e a estação pericial. Mire num item (alcance de ~2,6 m) e clique ou aperte **E** para abrir o painel compacto de ações físicas: fotografar, conferir lacre, deslacrar, isolar da rede, coletar memória, copiar volume cifrado montado, desligar, abrir gabinete, remover componentes (e descobrir mídias escondidas sob o dissipador), remover mídia, conectar bloqueador, adquirir imagem, conferir hash, isolar o celular, remover SIM, extrair, embalar, lacrar, etiquetar. A tela principal fica limpa: só o objeto na mira ganha rótulo; no canto, o ícone do tablet e o que está na mão.
* **Tablet do perito** — o perito carrega um tablet robusto (modelo 3D visto em mãos no canto da visão). A tecla **T** o abre sobre a cena, sem pausar nem alterar o caso, com quatro áreas: **Missão** (objetivo, ocorrência, quesitos, autorizações e tarefas conforme o nível: lista no 1 e 2, contagem no 3, nada no 4), **Objeto** (ficha de estado, preservação, mídias, produtos do exame e cuidados de manuseio — sem revelar o que ainda não foi descoberto), **Apoio** (mentor, resumo do POP, comandos e controles) e **Anotações** (criar, ver, editar e excluir, com marcador e horário do caso). As anotações ficam em `saves/anotacoes.json`, junto do caso salvo, e não entram nas evidências, na auditoria, no laudo nem na nota.
* **Passo a passo, com as ferramentas na mão** (casos novos, gerador 1.2.0) — nada "simplesmente acontece": cada atividade exige o objeto certo na mão, escolhido na **maleta pericial** (aproxime-se e aperte **E**): câmera para fotografar, ofício para conferir o lacre, tesoura para rompê-lo, chave Phillips para cada parafuso da tampa e do suporte do disco, cabo SATA/adaptador M.2/leitor SD para ligar a mídia ao bloqueador, cabo USB para ligar o bloqueador à estação, Faraday bag, papel-alumínio, ejetor de SIM, bateria externa, cabo USB-C ou Lightning conforme o aparelho, pendrive de coleta, saco de evidência, lacre numerado, etiquetas, lanterna e placas numeradas. Luvas de nitrilo e pulseira antiestática se vestem na maleta; sem elas a ação acontece, mas é falta (como na vida real). As etapas seguem a ordem física: parafusos → tampa → cabos de dados e energia → parafusos do suporte → mídia → cadeia mídia/bloqueador/estação → bloqueio → aquisição. O painel do objeto mostra só a próxima etapa possível, com o ícone da ferramenta que falta. **G** devolve a ferramenta à maleta. Casos e salvamentos antigos (gerador 1.0.0/1.1.0) seguem as regras clássicas.
* **Vestígios não digitais** — manchas de sangue seco, fio de cabelo, bituca de cigarro, copo descartável e pegada parcial, no material ou no local. O perito de informática não coleta: sinaliza com placa numerada e escala, fotografa e comunica à equipe de local. Tocar material com vestígio biológico suja as luvas; tocar outro objeto com elas é contaminação cruzada (troque as luvas na maleta).
* **Mãos e cenário** — as mãos do perito aparecem em primeira pessoa (com luvas azuis quando calçadas) segurando a ferramenta escolhida. Laboratório com placa de identificação, quadro branco com a fila de exames, cartaz da cadeia de custódia, cartaz de EPI, quadro de cortiça, relógio, extintor e lixeira de risco biológico; escritório com quadro, calendário, relógio, porta-retrato, cinzeiro e quadro de cortiça.
* **Tablet como ícone** — o tablet fica no canto da tela como ícone e só sai do bolso quando chamado (**T** ou clique).
* **Música ambiente** — jazz noir original (contrabaixo, vassourinhas, piano elétrico e sax), composta e sintetizada por script; liga/desliga no menu principal e volume próprio nas configurações.
* **Som** — ambiente próprio de cada lugar (laboratório: ar-condicionado, reator das luminárias e ventoinhas; escritório: trânsito pela janela, pássaros, relógio de parede; ventoinha do computador ao sentar na estação), passos que mudam com o piso (vinílico ou madeira), som ao tocar cada tipo de objeto e em cada ação pericial (obturador e flash, fita do lacre, parafusos e tampa do gabinete, conector SATA, bloqueador USB, disco girando na aquisição, bipes do hash, Faraday bag, papel-alumínio, bandeja do SIM, cabo de carga, embalagem, etiqueta…), o tablet saindo e voltando ao bolso, toques na tela, digitação e um aviso discreto quando o POP recusa a ação. Os sons das ações vêm do ponto do objeto na sala (esquerda/direita e distância). Os 50 sons são sintetizados por script (`tools/audio/gerar_sons.py`), sem gravações de terceiros. No Linux a saída é a do sistema (ALSA/PulseAudio/PipeWire); sem placa de som o jogo segue em silêncio e informa o motivo nas configurações.
* **Configurações** — botão **⚙ Configurações** (ou **Esc** no ambiente, e no menu principal): volumes geral, de ambiente, de efeitos e de interface, silenciar, som dos passos, testar som; cena 3D, sombras, economia de energia e campo de visão; sensibilidade do mouse e eixo invertido; escala da interface, dicas na tela e tablet nas mãos. Gravadas em `config.json` na pasta de dados.
* **Gráficos** — texturas procedurais (madeira, piso vinílico, tecido, metal escovado, teclado, etiquetas, vista da cidade pela janela…), iluminação por pixel com luminárias, luz da janela, especular e névoa leve, **sombras em tempo real** (mapa de sombras com filtragem PCF) e sombras de contato. Sombras e cena 3D podem ser desligadas em *Configurações* para máquinas fracas.
* **Estação Pericial** — um PC na cena 3D cujo monitor mostra a tela real da estação. Ao sentar nela você entra direto no **Perícia Linux 12.4** (sem senha), um Linux simulado (não é VM): painel com menu *Aplicativos* e lista de janelas, janelas móveis/redimensionáveis, **Terminal bash** (pipes, redirecionamento, `&&`/`||`, variáveis, curingas, histórico, Tab), **Arquivos**, **Editor de texto** e os aplicativos do simulador (Navegador da rede emulada, Documento pericial, Visualizador de evidências, Linha do tempo, Custódia, Manual, Mentor, Avaliação). Comandos gerais: `ls cd cat grep find stat file sha256sum mount umount lsblk dmesg df head tail wc sort uniq tree cp mv rm mkdir touch echo man…`, com `/dev`, `/proc`, `/mnt` e montagens somente leitura. As operações periciais são feitas com o comando `pericia <comando>` (ex.: `pericia imagem m-01`), que passa pelas regras do POP e pela auditoria.

Cada botão gera um **comando do terminal** (que aparece no terminal), então você aprende a sintaxe enquanto joga. Cada caso tem uma **expressão‑alvo** escondida nos vestígios; achá‑la não basta: a avaliação considera procedimento, preservação, rastreabilidade e fundamentação, e **perda de evidência limita a nota a 60%**.

## Situação do projeto

| Requisito | Situação |
|---|---|
| Cinco módulos (POP 5.01–5.05) | Implementados e testados (fluxos completos + falhas) |
| Gerador procedural reprodutível (semente + versão, 4 dificuldades) | Implementado; versões 1.0.0 e 1.1.0 suportadas (a 1.1.0 só acrescenta extensões, o conteúdo-base de cada ID não muda) |
| Sistema virtual de evidências (apagados, compostos, cifrados, setores ruins, fuso, OCR, metadados, hash) | Implementado; arquivos de sistema sintetizados sob demanda |
| Terminal próprio com ajuda interna | Implementado (`ajuda`, `ajuda fs`, `ajuda rede`, `ajuda laudo`) |
| Rede emulada coerente (DNS, WHOIS, rotas, páginas não indexadas, histórico, e‑mail com Received/SPF/DKIM/ARC, CGNAT) | Implementada; consistência verificada por testes |
| Evidências em nuvem com autorização judicial (requisição ao provedor × captura direta, bloqueio após tentativas) | Implementadas e testadas (5.01‑D4) |
| Cifragem integral por hardware + ordem de inicialização (BIOS) | Implementadas e testadas (5.01‑D3, gerador 1.1.0) |
| Casos‑limite de arquivos (apagado sobrescrito, setores ilegíveis, assinatura ≠ extensão) | Implementados (5.01/5.03 D2+, gerador 1.1.0) |
| Isolamento (sem rede, sem execução de comandos do SO) | Arquitetural; verificado por testes do código e das dependências |
| Eventos adversos (apagamento remoto, retirada de página, limite de tentativas de senha) | Implementados |
| Distinção apreensão × exame (5.05 gera **termo**, não laudo) | Implementada |
| Laudo/termo revisável, exportação **PDF** (gerador próprio) e **Markdown** | Implementados; PDF validado com Poppler |
| Destaque da expressão e ligação com a origem | Implementado (fundo amarelo no PDF, negrito no Markdown) |
| Coerência relatório × auditoria; laudo separa fatos observados (com fonte), limitações, hipóteses não confirmadas e procedimentos não realizados | Implementados (coerência por heurística) |
| Auditoria encadeada SHA‑256 | Implementada (formato 2, sem ambiguidade entre campos); adulteração, remoção e reordenação detectadas |
| Avaliação explicada + mentor adaptativo | Implementados |
| Persistência (continuar caso, histórico, configurações) | Replay determinístico; gravação atômica; ao carregar, aponta a primeira entrada de auditoria que divergiu; salvamentos da v0.3 continuam carregando |
| Interface 3D em primeira pessoa (colisão, mira, interação) + texturas + sombras + modo 2D leve + economia de energia | Implementados |
| Interação física: bancada de ferramentas organizável, etapas na ordem com a ferramenta certa, indicadores de uso, cabos ponta a ponta, exame de perto, objetivo marcado | Implementada; um teste conclui todas as etapas de 80 casos (5 módulos × 4 dificuldades) sem travar na ordem |
| Linux simulado na estação (desktop, terminal bash, arquivos, editor com cursor) | Implementado; replay do terminal reproduz a auditoria (testado) |
| Preparação para modelos/texturas externos (`assets/`) | Substituição de texturas PNG/TGA validada; OBJ/MTL lido e validado; inventário com SHA‑256 dos originais |
| Regras realistas do aparelho móvel: bateria em AFU, IMEI físico × lógico, ligar/desligar | Implementadas e testadas |
| Tela simulada do celular (FoneOS: tela bloqueada BFU/AFU, emergência/*#06#, início, mensagens, telefone, contatos, galeria com EXIF, arquivos, relógio, configurações; navegação auditada e sujeita às regras dos POPs) | Implementada e testada (9 testes). Sem teclado de desbloqueio na tela: o acesso depende do estado do aparelho, da senha documentada ou da extração |
| **Linux — Ubuntu 24.04 (glibc 2.39)** | Compilado, executado, testado e medido (Xvfb + Mesa llvmpipe, sem GPU) |
| **Linux — Ubuntu 22.04 (glibc 2.35)** | Binário executado sob Xvfb e capturas conferidas (ver docs) |
| **Arch Linux / Omarchy (Hyprland, Wayland)** | `install.sh` e `packaging/arch/PKGBUILD` preparados; **não testado numa máquina Arch** — o ambiente de desenvolvimento não tem Arch nem Wayland. Roda por XWayland (padrão do Omarchy); `PERICIA_WAYLAND=1` tenta Wayland nativo. Som exige `alsa-lib` + `pipewire-alsa` |
| **Linux anterior à glibc 2.34** (ex.: Ubuntu 20.04) | **Não suportado pelo binário pronto** — compilar no próprio sistema |
| **Windows (x86_64) / macOS (Apple Silicon e Intel)** | Compilados e testados automaticamente pelo GitHub Actions a cada versão (`.github/workflows/compilar.yml`); som por WASAPI/CoreAudio (cpal). **Ainda não jogados numa máquina Windows ou Mac real** |

**Pendências conhecidas**: RAID/JBOD não é modelado (exigiria mídias compostas no sistema de evidências — mudança estrutural, deixada para decisão); o PIN do chip SIM (3 tentativas/PUK 10) consta do modelo e do manual, mas não tem comando; modelos glTF/FBX ainda não são carregados (aguardam os arquivos e a avaliação); testes de interface são por roteiro + captura de tela (sem asserções automáticas sobre pixels).

## Baixar

Na página **Releases** do repositório há um pacote por sistema:

| Sistema | Arquivo | Como abrir |
|---|---|---|
| Windows 10/11 | `pericia-sim-<versão>-windows-x86_64.zip` | extraia a pasta e abra `pericia-sim.exe` (SmartScreen: *Mais informações → Executar assim mesmo*) |
| macOS 11+ | `pericia-sim-<versão>-macos-universal.zip` | abra `Perícia Sim.app` com o botão direito → *Abrir* (app sem assinatura da Apple) |
| Linux x86_64 | `pericia-sim-<versão>-linux-x86_64.tar.gz` | veja abaixo |

## Executar (Linux x86_64)

```bash
tar xzf pericia-sim-0.11.0-linux-x86_64.tar.gz
cd pericia-sim-0.11.0-linux-x86_64
./pericia-sim          # rodar direto
./install.sh           # ou instalar no menu de aplicativos (sem sudo)
```

Requer apenas as bibliotecas OpenGL/X11 do sistema (presentes em qualquer desktop Linux). Funciona com o driver por software do Mesa (`llvmpipe`) quando não há GPU.

### Arch Linux / Omarchy

```bash
sudo pacman -S --needed libgl libx11 libxi libxkbcommon xorg-xwayland alsa-lib pipewire-alsa
./install.sh                        # instala em ~/.local (atalho no menu, comando pericia-sim)
# ou, a partir do código-fonte:
cd packaging/arch && makepkg -si    # compila e instala como pacote do pacman
```

No Hyprland o jogo abre por XWayland (é o padrão do Omarchy). Para tentar o backend Wayland nativo: `PERICIA_WAYLAND=1 pericia-sim`. Sem `pipewire-alsa` o jogo funciona, mas fica sem som.

## Compilar a partir do código (Linux, Windows, macOS)

1. Instale o Rust (https://rustup.rs).
2. Linux: `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev` (ou equivalentes da sua distribuição).
3. Na pasta do projeto:

```bash
cargo test --workspace              # 118 testes (motor, passo a passo, geração procedural, Linux simulado, celular, quadro, replay, assets)
cargo run --release -p pericia_app  # abre o simulador
```

No Windows e no macOS o mesmo comando gera o executável nativo (`target/release/pericia-sim(.exe)`).

## Como jogar (5 minutos)

1. No menu, escolha o **POP**, a **dificuldade** (comece por *1 · Guiado*) e clique em **Iniciar novo caso**. Para repetir um caso, digite o ID (ex.: `C501-D2-00000007`).
2. Ajuste volume e controles em **⚙ Configurações**, se quiser. Leia o caso: tecla **T** (tablet → **Missão**) ou, na Estação, aplicativo **Caso**. O mentor fica no tablet, em **Apoio**.
3. No **Ambiente**: **WASD** anda (Shift corre), olhe com o **mouse** (travado no centro; o ponto é a mira). **Esc** ou **Tab** soltam o mouse e um clique na cena trava de novo; ele fica livre sozinho no tablet, no quadro, no celular e na estação. **+ / −** ajustam a sensibilidade (também em *Configurações → Controles*, de 0,1× a 5×).
   * **Bancada de ferramentas** (laboratório: junto à parede da frente; local de busca: a mesa de apoio): cada ferramenta, a caixa de luvas e a pulseira são objetos soltos. **E** pega, **G** larga a ferramenta da mão onde você mirar (bancada, mesa de exame ou mesa da estação) — organize como quiser, a arrumação fica salva.
   * **Usar, peça por peça**: com a ferramenta certa na mão, mire **a peça** e aperte **E**. Cada parafuso da tampa (na traseira do gabinete; embaixo do notebook, que fica fechado e virado depois de desligado e inspecionado), a tampa, o lacre, o botão de energia, a porta USB, a porta de rede, o conector de dados e o de energia de cada disco, cada parafuso do suporte e a própria mídia são alvos separados — o parafuso sai e some, o cabo desconectado fica solto. No celular: a porta de dados/carga, a gaveta do chip e a câmera frontal. Fotografar, inspecionar, isolar e embalar continuam no objeto inteiro. Fora de ordem, o jogo diz qual etapa falta, com que ferramenta e **em qual peça**. Com uma ferramenta na mão, **losangos amarelos** marcam cada peça onde ela serve agora.
   * **Cabos, de porta a porta**: **E** no conector da mídia e depois **E** na porta certa do bloqueador (SATA, M.2, leitor SD ou entrada USB, na traseira e na lateral); o cabo USB vai da saída do bloqueador à porta USB frontal do gabinete da estação. A chave de proteção contra escrita fica em cima do bloqueador. Os cabos ligados aparecem na cena, de ponta a ponta.
   * **Painel de tarefas** (canto superior esquerdo, semitransparente): o objetivo marcado e as etapas do objeto em foco, com a ferramenta de cada uma. **O** mostra/oculta.
   * **Examinar**: **F** aproxima a câmera do objeto (o mouse gira, a roda aproxima) e mostra o que está escrito nele — fabricante, nº de série, lacre, IMEI, estado. No celular, **Enter** mostra a tela.
   * **Objetivo**: no tablet → **Objeto**, cada objeto tem a lista de etapas na ordem; clique numa para marcá-la — ela aparece discretamente no canto da tela, com a ferramenta, e avança sozinha.
4. Adquira a mídia (bloqueador → imagem → verificar hash) ou extraia o celular.
5. Vá até a **Estação Pericial** e clique no monitor (ou use a aba no topo) — a sessão já abre com um terminal. Para voltar, **◄ Levantar da estação**. No Terminal: `lsblk`, `pericia imagem m-01`, `sudo mkdir -p /mnt/m01`, `sudo mount -o ro,loop ~/casos/<ID>/M-01.dd /mnt/m01`, `grep -ril <termo> /mnt/m01`, `stat`, `sha256sum`. Use também **Arquivos**, **Navegador** e **Linha do tempo** (menu *Aplicativos*).
6. Quando achar a expressão, registre com `achei <expressão>`; no **Documento**, use *Pré‑preencher*, revise, escreva a conclusão separando fato de inferência, *Verificar coerência* e *Emitir e finalizar*.
7. **Quadro de provas** (parede esquerda, ou mire nele e aperte E): cada foto tirada com a câmera na cena vira uma foto de verdade no quadro; acrescente bilhetes, anotações do tablet e ligue tudo com barbante vermelho. O quadro também aparece na parede, em 3D.
8. Leia a **Avaliação**: cada ponto perdido traz o motivo.

O terminal do simulador fica oculto: **F2** (ou **'**) abre e fecha, **Esc** ou **✕ Fechar** escondem. Atalhos do terminal: ↑/↓ histórico, `limpar`, Ctrl+V cola. No editor e nos campos do documento: setas, Home/End, Delete e clique posicionam o cursor.

## Modelos 3D e texturas

Os móveis, equipamentos e objetos de trabalho (cadeiras, mesas, monitor, gabinete, notebook, celular, bancada, armário, estantes, gaveteiro, planta, câmera, bloqueador de escrita, kit de embalagem, luminária, teclado, mouse, HD, SSD e pendrive) são modelos 3D feitos no **Blender 5.2** por script (`tools/blender/gerar_modelos.py`), com a fonte editável em `assets/blender/modelos.blend`. Sem a pasta `assets/`, o jogo volta à geometria procedural. Veja `assets/LEIA-ME.md`. Resumo: texturas PNG/TGA em `assets/textures/<nome>.png` substituem as procedurais (validadas: potência de 2, até 2048 px); modelos OBJ/MTL em `assets/models/` são lidos e validados (relatório em `assets_relatorio.txt` na pasta de dados). O inventário dos arquivos recebidos sai de `cargo run --release -p pericia_core --example asset_inventory -- <pasta> <saida.md>` (somente leitura, com SHA‑256 de cada original).

## Arquivos gerados

Em `~/.local/share/pericia-sim` (Linux), `%APPDATA%\pericia-sim` (Windows) ou `~/Library/Application Support/pericia-sim` (macOS); pode ser trocado pela variável `PERICIA_DATA`.

* `saves/atual.json` — caso em andamento (ID + comandos).
* `historico.json` — casos concluídos.
* `exportados/<ID>.pdf` e `.md` — documentos emitidos.

## Arquitetura

```
crates/core   motor puro em Rust (apenas serde): casos, evidências, rede emulada,
              regras dos POPs, auditoria, avaliação, laudo/termo, PDF e Markdown
crates/app    interface macroquad: renderizador próprio (shader GLSL, sombras, texturas
              procedurais), mundo em primeira pessoa, desktop Linux simulado
docs/         análise técnica, design da simulação, desempenho, ADRs, matriz de rastreabilidade
```

Ferramentas de apoio (`cargo run --release -p pericia_core --example ...`): `bench` (desempenho), `demo_pdf` (laudo de exemplo), `gen_matrix` (regenera a matriz), `case_info <ID>` (dados de um caso, para instrutores).

## Documentação

* `docs/01-analise-tecnica.md` — materiais lidos, requisitos por POP, ambiguidades, arquitetura.
* `docs/02-design-da-simulacao.md` — design pelo método ISAGA (matriz componentes × elementos de jogo).
* `docs/03-desempenho.md` — medições reais (sem GPU) e técnicas de economia.
* `docs/matriz-rastreabilidade.md` — fonte no POP → comportamento → teste → limitação.
* `docs/adr/` — decisões de arquitetura (tecnologia; persistência por replay).

## Limites da simulação

Recuperação, extração móvel, memória volátil e rede são **modelos didáticos**, coerentes com o estado do caso, mas não reproduzem ferramentas reais. Os resumos dos POPs no manual interno são paráfrases para estudo; trechos marcados “[complemento]” não vêm do POP. O estilo do PDF é inspirado em convenções ABNT, **sem** alegar conformidade normativa. Nada aqui é documento oficial.

Fontes DejaVu: licença livre (ver `crates/app/assets/DEJAVU-LICENSE.txt`).
