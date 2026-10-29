# Perícia Sim — Simulador de Informática Forense

Simulador de treinamento em perícia de informática, no estilo “hacking simulator”, baseado nos **POPs de Perícia Criminal 2024 — Informática Forense, vol. 5 (SENASP/MJSP)**. Escrito em **Rust**, com cena 3D leve que roda sem placa de vídeo dedicada.

> Todo o conteúdo dos casos é **fictício**. A “internet” do jogo é **emulada** dentro do programa: nenhuma ferramenta investigativa acessa a rede real, e o terminal **não** executa comandos do sistema operacional.

## O que é

Duas áreas, como pedido:

* **Área de Hardware (3D)** — bancada de laboratório ou local de busca (escritório). Você vê os equipamentos (gabinetes, notebook, celular, HDs, pendrives), a câmera, o bloqueador de escrita, Faraday bag, lacres e a estação pericial ao fundo. Clique num item para ver o estado e as ações físicas disponíveis: fotografar, conferir lacre, deslacrar, isolar da rede, coletar memória, copiar volume cifrado montado, desligar, abrir gabinete, remover componentes (e descobrir mídias escondidas sob o dissipador), remover mídia, conectar bloqueador, adquirir imagem, conferir hash, isolar o celular, remover SIM, extrair, embalar, lacrar, etiquetar.
* **Estação Pericial** — um PC na cena 3D; ao usá‑lo você entra no **PeritOS**: terminal, navegador de evidências (sobre as imagens adquiridas), navegador da rede emulada (DNS, WHOIS, traceroute, páginas, histórico, cabeçalho de e‑mail, logs do provedor/CGNAT), cadeia de custódia, linha do tempo, editor do documento (laudo ou termo de apreensão), manual dos POPs, mentor e avaliação.

Cada botão gera um **comando do terminal** (que aparece no terminal), então você aprende a sintaxe enquanto joga. Cada caso tem uma **expressão‑alvo** escondida nos vestígios; achá‑la não basta: a avaliação considera procedimento, preservação, rastreabilidade e fundamentação, e **perda de evidência limita a nota a 60%**.

## Situação do projeto

| Requisito | Situação |
|---|---|
| Cinco módulos (POP 5.01–5.05) | Implementados e testados (fluxos completos + falhas) |
| Gerador procedural reprodutível (semente + versão, 4 dificuldades) | Implementado; testado em 5 módulos × 4 níveis × 40 sementes |
| Sistema virtual de evidências (apagados, compostos, cifrados, setores ruins, fuso, OCR, metadados, hash) | Implementado; arquivos de sistema sintetizados sob demanda |
| Terminal próprio com ajuda interna | Implementado (`ajuda`, `ajuda fs`, `ajuda rede`, `ajuda laudo`) |
| Rede emulada coerente (DNS, WHOIS, rotas, páginas não indexadas, histórico, e‑mail com Received/SPF/DKIM, CGNAT) | Implementada; consistência verificada por testes |
| Isolamento (sem rede, sem execução de comandos do SO) | Arquitetural; verificado por testes do código e das dependências |
| Eventos adversos (apagamento remoto, retirada de página, limite de tentativas de senha) | Implementados |
| Distinção apreensão × exame (5.05 gera **termo**, não laudo) | Implementada |
| Laudo/termo revisável, exportação **PDF** (gerador próprio) e **Markdown** | Implementados; PDF validado com Poppler |
| Destaque da expressão e ligação com a origem | Implementado (fundo amarelo no PDF, negrito no Markdown) |
| Coerência relatório × auditoria | Implementada (heurística; ver limitações) |
| Auditoria encadeada SHA‑256 | Implementada; adulteração detectada |
| Avaliação explicada + mentor adaptativo | Implementados |
| Persistência (continuar caso, histórico, configurações) | Implementada por replay determinístico |
| Interface 3D + modo 2D leve + economia de energia | Implementados |
| **Linux** | Compilado, executado e medido (OpenGL por software, sem GPU) |
| **Windows / macOS** | **Não testados** — o código não usa nada específico de Linux; compile nativamente (abaixo) |

**Pendências conhecidas** (não implementadas ou parciais): acesso à nuvem do caso 5.01‑D4 ainda não exposto no terminal; cifragem integral por hardware existe no motor mas não é sorteada pelo gerador; bateria do celular não se esgota com o tempo; RAID/JBOD, ordem de boot na BIOS e ARC não são modelados; o editor de texto só edita no final do campo; testes de interface são por roteiro + captura de tela (não há asserções automáticas sobre pixels).

## Executar (Linux x86_64)

```bash
tar xzf pericia-sim-0.1.0-linux-x86_64.tar.gz
cd pericia-sim-0.1.0-linux-x86_64
./pericia-sim
```

Requer apenas as bibliotecas OpenGL/X11 do sistema (presentes em qualquer desktop Linux). Funciona com o driver por software do Mesa (`llvmpipe`) quando não há GPU.

## Compilar a partir do código (Linux, Windows, macOS)

1. Instale o Rust (https://rustup.rs).
2. Linux: `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev` (ou equivalentes da sua distribuição).
3. Na pasta do projeto:

```bash
cargo test -p pericia_core          # 37 testes do motor
cargo run --release -p pericia_app  # abre o simulador
```

No Windows e no macOS o mesmo comando gera o executável nativo (`target/release/pericia-sim(.exe)`).

## Como jogar (5 minutos)

1. No menu, escolha o **POP**, a **dificuldade** (comece por *1 · Guiado*) e clique em **Iniciar novo caso**. Para repetir um caso, digite o ID (ex.: `C501-D2-00000007`).
2. Leia o caso: botão **Mentor** ou, na Estação, aplicativo **Caso** (histórico, objetivo, quesitos, autorização legal).
3. Na **Área de Hardware**: botão direito gira a câmera, roda aproxima, clique seleciona. Use as ações do painel direito na ordem do POP.
4. Adquira a mídia (bloqueador → imagem → verificar hash) ou extraia o celular.
5. Na **Estação Pericial** → *Usar a estação*: examine em **Evidências**, investigue em **Navegador**, cruze em **Linha do tempo**.
6. Quando achar a expressão, registre com `achei <expressão>`; no **Documento**, use *Pré‑preencher*, revise, escreva a conclusão separando fato de inferência, *Verificar coerência* e *Emitir e finalizar*.
7. Leia a **Avaliação**: cada ponto perdido traz o motivo.

Atalhos do terminal: ↑/↓ histórico, `limpar`, Ctrl+V cola.

## Arquivos gerados

Em `~/.local/share/pericia-sim` (Linux), `%APPDATA%\pericia-sim` (Windows) ou `~/Library/Application Support/pericia-sim` (macOS); pode ser trocado pela variável `PERICIA_DATA`.

* `saves/atual.json` — caso em andamento (ID + comandos).
* `historico.json` — casos concluídos.
* `exportados/<ID>.pdf` e `.md` — documentos emitidos.

## Arquitetura

```
crates/core   motor puro em Rust (apenas serde): casos, evidências, rede emulada,
              regras dos POPs, auditoria, avaliação, laudo/termo, PDF e Markdown
crates/app    interface macroquad: cena 3D procedural + PeritOS
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
