# Perícia Sim — Simulador de Informática Forense

Simulador de treinamento em perícia de informática, no estilo “hacking simulator”, baseado nos **POPs de Perícia Criminal 2024 — Informática Forense, vol. 5 (SENASP/MJSP)**. Escrito em **Rust**, em **primeira pessoa**, com iluminação por pixel, sombras em tempo real e texturas geradas proceduralmente — roda inclusive sem placa de vídeo dedicada.

> Todo o conteúdo dos casos é **fictício**. A “internet” do jogo é **emulada** dentro do programa: nenhuma ferramenta investigativa acessa a rede real, e o terminal **não** executa comandos do sistema operacional.

## O que é

Duas áreas, como pedido:

* **Ambiente em primeira pessoa (3D)** — laboratório pericial ou local de busca (escritório), percorridos a pé, sem personagem visível. Bancada antiestática, gabinetes, notebook, celular, HDs, pendrives, câmera, bloqueador de escrita, Faraday bag, lacres, armário, estante e a estação pericial. Mire num item (alcance de ~2,6 m) e clique ou aperte **E** para ver o estado e as ações físicas: fotografar, conferir lacre, deslacrar, isolar da rede, coletar memória, copiar volume cifrado montado, desligar, abrir gabinete, remover componentes (e descobrir mídias escondidas sob o dissipador), remover mídia, conectar bloqueador, adquirir imagem, conferir hash, isolar o celular, remover SIM, extrair, embalar, lacrar, etiquetar. O botão **☰ Itens e ações** lista tudo sem precisar andar.
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
| Linux simulado na estação (desktop, terminal bash, arquivos, editor com cursor) | Implementado; replay do terminal reproduz a auditoria (testado) |
| Preparação para modelos/texturas externos (`assets/`) | Substituição de texturas PNG/TGA validada; OBJ/MTL lido e validado; inventário com SHA‑256 dos originais |
| Regras realistas do aparelho móvel: bateria em AFU, IMEI físico × lógico, ligar/desligar | Implementadas e testadas |
| Tela simulada do celular (FoneOS: tela bloqueada BFU/AFU, emergência/*#06#, início, mensagens, telefone, contatos, galeria com EXIF, arquivos, relógio, configurações; navegação auditada e sujeita às regras dos POPs) | Implementada e testada (9 testes). Sem teclado de desbloqueio na tela: o acesso depende do estado do aparelho, da senha documentada ou da extração |
| **Linux — Ubuntu 24.04 (glibc 2.39)** | Compilado, executado, testado e medido (Xvfb + Mesa llvmpipe, sem GPU) |
| **Linux — Ubuntu 22.04 (glibc 2.35)** | Binário executado sob Xvfb e capturas conferidas (ver docs) |
| **Linux anterior à glibc 2.34** (ex.: Ubuntu 20.04) | **Não suportado pelo binário pronto** — compilar no próprio sistema |
| **Windows / macOS** | **Não compilados nem testados** (o ambiente de desenvolvimento não tem acesso às ferramentas de compilação cruzada); o código não usa nada específico de Linux |

**Pendências conhecidas**: RAID/JBOD não é modelado (exigiria mídias compostas no sistema de evidências — mudança estrutural, deixada para decisão); o PIN do chip SIM (3 tentativas/PUK 10) consta do modelo e do manual, mas não tem comando; modelos glTF/FBX ainda não são carregados (aguardam os arquivos e a avaliação); testes de interface são por roteiro + captura de tela (sem asserções automáticas sobre pixels).

## Executar (Linux x86_64)

```bash
tar xzf pericia-sim-0.4.0-linux-x86_64.tar.gz
cd pericia-sim-0.4.0-linux-x86_64
./pericia-sim          # rodar direto
./install.sh           # ou instalar no menu de aplicativos (sem sudo)
```

Requer apenas as bibliotecas OpenGL/X11 do sistema (presentes em qualquer desktop Linux). Funciona com o driver por software do Mesa (`llvmpipe`) quando não há GPU.

## Compilar a partir do código (Linux, Windows, macOS)

1. Instale o Rust (https://rustup.rs).
2. Linux: `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev` (ou equivalentes da sua distribuição).
3. Na pasta do projeto:

```bash
cargo test --workspace              # 87 testes (motor, Linux simulado, celular, nuvem, extensões, replay, assets)
cargo run --release -p pericia_app  # abre o simulador
```

No Windows e no macOS o mesmo comando gera o executável nativo (`target/release/pericia-sim(.exe)`).

## Como jogar (5 minutos)

1. No menu, escolha o **POP**, a **dificuldade** (comece por *1 · Guiado*) e clique em **Iniciar novo caso**. Para repetir um caso, digite o ID (ex.: `C501-D2-00000007`).
2. Leia o caso: botão **Mentor** ou, na Estação, aplicativo **Caso** (histórico, objetivo, quesitos, autorização legal).
3. No **Ambiente**: **WASD** anda (Shift corre), segure o **botão direito** (ou aperte **Tab** para capturar o mouse) para olhar, mire e clique/**E** para interagir. Use as ações do painel na ordem do POP.
4. Adquira a mídia (bloqueador → imagem → verificar hash) ou extraia o celular.
5. Vá até a **Estação Pericial** e clique no monitor (ou use a aba no topo) — a sessão já abre com um terminal. Para voltar, **◄ Levantar da estação**. No Terminal: `lsblk`, `pericia imagem m-01`, `sudo mkdir -p /mnt/m01`, `sudo mount -o ro,loop ~/casos/<ID>/M-01.dd /mnt/m01`, `grep -ril <termo> /mnt/m01`, `stat`, `sha256sum`. Use também **Arquivos**, **Navegador** e **Linha do tempo** (menu *Aplicativos*).
6. Quando achar a expressão, registre com `achei <expressão>`; no **Documento**, use *Pré‑preencher*, revise, escreva a conclusão separando fato de inferência, *Verificar coerência* e *Emitir e finalizar*.
7. Leia a **Avaliação**: cada ponto perdido traz o motivo.

Atalhos do terminal: ↑/↓ histórico, `limpar`, Ctrl+V cola. No editor e nos campos do documento: setas, Home/End, Delete e clique posicionam o cursor.

## Modelos 3D e texturas externos

Veja `assets/LEIA-ME.md`. Resumo: texturas PNG/TGA em `assets/textures/<nome>.png` substituem as procedurais (validadas: potência de 2, até 2048 px); modelos OBJ/MTL em `assets/models/` são lidos e validados (relatório em `assets_relatorio.txt` na pasta de dados). O inventário dos arquivos recebidos sai de `cargo run --release -p pericia_core --example asset_inventory -- <pasta> <saida.md>` (somente leitura, com SHA‑256 de cada original).

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
