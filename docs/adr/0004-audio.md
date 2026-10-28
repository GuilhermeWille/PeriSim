# ADR 0004 — Áudio: mixer próprio e ALSA carregada em tempo de execução

**Data:** 28/10/2026 · **Situação:** aceita

## Contexto

O jogo precisava de som ambiente, efeitos posicionados e volumes por categoria. O macroquad
oferece áudio (`quad-snd`), mas a sua thread de áudio no Linux encerra com `panic!` quando não
há dispositivo de som — e o perfil de release usa `panic = "abort"`, o que derrubaria o jogo
inteiro em máquinas virtuais, sessões remotas ou computadores sem placa de som. Além disso, a
biblioteca liga o executável à `libasound` na compilação.

## Decisão

* Mixer próprio (`crates/app/src/audio.rs`) numa thread: vozes com ganho esquerdo/direito,
  variação de altura, laços com fusão de entrada/saída e três categorias (ambiente, efeitos,
  interface) mais volume geral e mudo, com volume suavizado por bloco e limitador suave.
* Saída Linux por `dlopen("libasound.so.2")`, abrindo `default`, `pipewire` ou `pulse` sem
  bloquear. Qualquer falha vira uma mensagem nas configurações; o jogo continua em silêncio.
* Gravação para validação sem placa de som: `PERICIA_AUDIO_WAV` (tempo real) e
  `PERICIA_AUDIO_OFFLINE` (um quadro = 1/30 s, usado com `PERICIA_VIDEO` para vídeos de
  demonstração com som sincronizado).
* Sons sintetizados por script (`tools/audio/gerar_sons.py`), reprodutíveis, sem direitos de
  terceiros.

## Consequências

* O executável não depende de `libasound` para iniciar.
* Windows e macOS compilam, mas ainda sem saída de som (informado nas configurações).
* A posição dos sons é estéreo simples (pan e atenuação por distância), sem HRTF nem
  reverberação dependente da sala em tempo real — a sala curta já vem nos arquivos.
