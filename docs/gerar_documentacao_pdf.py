# Gera a documentação do Perícia Sim em PDF, com formatação inspirada na ABNT NBR 14724.
import re, sys, os
from xml.sax.saxutils import escape
from reportlab.lib.pagesizes import A4
from reportlab.lib.units import cm
from reportlab.lib import colors
from reportlab.lib.enums import TA_JUSTIFY, TA_CENTER, TA_LEFT, TA_RIGHT
from reportlab.lib.styles import ParagraphStyle
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import (BaseDocTemplate, PageTemplate, Frame, Paragraph, Spacer, PageBreak,
                                Image, Table, TableStyle, KeepTogether, Flowable, CondPageBreak)
from reportlab.platypus.tableofcontents import TableOfContents

D = sys.argv[1]          # pasta das imagens
ROOT = sys.argv[2]       # raiz do projeto
OUT = sys.argv[3]

F = '/usr/share/fonts/truetype/liberation/'
pdfmetrics.registerFont(TTFont('Serif', F + 'LiberationSerif-Regular.ttf'))
pdfmetrics.registerFont(TTFont('Serif-B', F + 'LiberationSerif-Bold.ttf'))
pdfmetrics.registerFont(TTFont('Serif-I', F + 'LiberationSerif-Italic.ttf'))
pdfmetrics.registerFont(TTFont('Serif-BI', F + 'LiberationSerif-BoldItalic.ttf'))
pdfmetrics.registerFont(TTFont('Mono', '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'))
from reportlab.pdfbase.pdfmetrics import registerFontFamily
registerFontFamily('Serif', normal='Serif', bold='Serif-B', italic='Serif-I', boldItalic='Serif-BI')

AUTOR = 'Guilherme Wille'
TITULO = 'PERÍCIA SIM'
SUBTITULO = 'simulador de treinamento em informática forense baseado nos POPs SENASP, volume 5'
LOCAL = 'Brasil'
ANO = '2026'
VERSAO = '0.4.0'

LEAD = 18  # 1,5 linha para 12 pt
body = ParagraphStyle('body', fontName='Serif', fontSize=12, leading=LEAD, alignment=TA_JUSTIFY, firstLineIndent=1.25 * cm, spaceAfter=0)
body_ni = ParagraphStyle('body_ni', parent=body, firstLineIndent=0)
bullet = ParagraphStyle('bullet', parent=body, firstLineIndent=0, leftIndent=0.9 * cm, bulletIndent=0.3 * cm)
h1 = ParagraphStyle('h1', fontName='Serif-B', fontSize=12, leading=LEAD, spaceBefore=0, spaceAfter=LEAD, alignment=TA_LEFT)
h2 = ParagraphStyle('h2', fontName='Serif', fontSize=12, leading=LEAD, spaceBefore=LEAD, spaceAfter=LEAD, alignment=TA_LEFT)
h3 = ParagraphStyle('h3', fontName='Serif-B', fontSize=12, leading=LEAD, spaceBefore=LEAD, spaceAfter=LEAD / 2, alignment=TA_LEFT)
center_b = ParagraphStyle('cb', fontName='Serif-B', fontSize=12, leading=LEAD, alignment=TA_CENTER)
center = ParagraphStyle('c', fontName='Serif', fontSize=12, leading=LEAD, alignment=TA_CENTER)
cap = ParagraphStyle('cap', fontName='Serif', fontSize=10, leading=12, alignment=TA_CENTER, spaceBefore=LEAD / 2, spaceAfter=4)
src = ParagraphStyle('src', fontName='Serif', fontSize=10, leading=12, alignment=TA_CENTER, spaceBefore=4, spaceAfter=LEAD)
cell = ParagraphStyle('cell', fontName='Serif', fontSize=10, leading=12, alignment=TA_LEFT)
cellb = ParagraphStyle('cellb', parent=cell, fontName='Serif-B')
cell8 = ParagraphStyle('cell8', fontName='Serif', fontSize=8, leading=9.6, alignment=TA_LEFT)
cell8b = ParagraphStyle('cell8b', parent=cell8, fontName='Serif-B')
code = ParagraphStyle('code', fontName='Mono', fontSize=9, leading=12, leftIndent=0.6 * cm, spaceBefore=6, spaceAfter=10, backColor=colors.HexColor('#f2f2f2'), borderPadding=5)
ref = ParagraphStyle('ref', fontName='Serif', fontSize=12, leading=14, alignment=TA_LEFT, spaceAfter=14)
nature = ParagraphStyle('nat', fontName='Serif', fontSize=10, leading=12, alignment=TA_JUSTIFY, leftIndent=8 * cm)

TEXT_W = A4[0] - 5 * cm


class Marker(Flowable):
    """Liga a numeração visível das páginas (ABNT: a partir da parte textual)."""
    def __init__(self):
        super().__init__()
        self.width = self.height = 0

    def draw(self):
        pass


class ListOf(TableOfContents):
    def __init__(self, key, **kw):
        super().__init__(**kw)
        self.key = key

    def notify(self, kind, stuff):
        if kind == self.key:
            self.addEntry(*stuff)


class Doc(BaseDocTemplate):
    def __init__(self, fn, **kw):
        super().__init__(fn, **kw)
        self.show_num = False
        frame = Frame(3 * cm, 2 * cm, A4[0] - 5 * cm, A4[1] - 5 * cm, id='f', leftPadding=0, rightPadding=0, topPadding=0, bottomPadding=0)
        self.addPageTemplates([PageTemplate(id='p', frames=[frame], onPageEnd=self.on_page)])
        self.fig_n = 0
        self.q_n = 0

    def beforeDocument(self):
        self.show_num = False

    def on_page(self, canv, doc):
        # capa não é contada; a contagem começa na folha de rosto
        n = doc.page - 1
        if self.show_num and n >= 1:
            canv.setFont('Serif', 10)
            canv.drawRightString(A4[0] - 2 * cm, A4[1] - 2 * cm + 0.3 * cm, str(n))

    def afterFlowable(self, f):
        if isinstance(f, Marker):
            self.show_num = True
        if isinstance(f, Paragraph):
            st = f.style.name
            txt = f.getPlainText()
            if st == 'h1u' and not (txt.startswith('REFER') or txt.startswith('APÊNDICE')):
                return
            if st in ('h1', 'h2', 'h3', 'h1u'):
                lvl = {'h1': 0, 'h1u': 0, 'h2': 1, 'h3': 2}[st]
                key = 'h%d' % id(f)
                self.canv.bookmarkPage(key)
                self.canv.addOutlineEntry(txt, key, level=lvl, closed=lvl > 0)
                self.notify('TOCEntry', (lvl, txt, self.page - 1, key))
            elif st == 'cap':
                if txt.startswith('Figura'):
                    self.notify('FigEntry', (0, txt, self.page - 1))
                elif txt.startswith('Quadro'):
                    self.notify('QEntry', (0, txt, self.page - 1))


h1u = ParagraphStyle('h1u', parent=h1, alignment=TA_CENTER)  # títulos sem indicativo numérico

story = []
P = lambda t, s=body: story.append(Paragraph(t, s))


def bl(items):
    for it in items:
        story.append(Paragraph(it, bullet, bulletText='•'))


fig_counter = [0]
q_counter = [0]


def figure(path, title, w=TEXT_W, fonte='Elaborada pelo autor (2026), captura de tela do simulador.'):
    from PIL import Image as PI
    im = PI.open(path)
    h = w * im.height / im.width
    maxh = 15.5 * cm
    if h > maxh:
        w, h = w * maxh / h, maxh
    fig_counter[0] += 1
    story.append(KeepTogether([
        Paragraph('Figura %d – %s' % (fig_counter[0], title), cap),
        Image(path, width=w, height=h),
        Paragraph('Fonte: ' + fonte, src),
    ]))


def quadro(title, header, rows, widths, small=False, fonte='Elaborado pelo autor (2026).', repeat=True):
    q_counter[0] += 1
    cs, csb = (cell8, cell8b) if small else (cell, cellb)
    data = [[Paragraph(h, csb) for h in header]] + [[Paragraph(c, cs) for c in r] for r in rows]
    t = Table(data, colWidths=[w * TEXT_W for w in widths], repeatRows=1 if repeat else 0)
    t.setStyle(TableStyle([
        ('GRID', (0, 0), (-1, -1), 0.5, colors.black),
        ('VALIGN', (0, 0), (-1, -1), 'TOP'),
        ('BACKGROUND', (0, 0), (-1, 0), colors.HexColor('#e6e6e6')),
        ('LEFTPADDING', (0, 0), (-1, -1), 4), ('RIGHTPADDING', (0, 0), (-1, -1), 4),
        ('TOPPADDING', (0, 0), (-1, -1), 3), ('BOTTOMPADDING', (0, 0), (-1, -1), 3),
    ]))
    story.append(CondPageBreak(4 * cm))
    story.append(Paragraph('Quadro %d – %s' % (q_counter[0], title), cap))
    story.append(t)
    story.append(Paragraph('Fonte: ' + fonte, src))


def c(s):
    """código inline"""
    return '<font name="Mono" size="10">%s</font>' % escape(s)


# ------------------------------------------------------------------ CAPA
story.append(Paragraph(AUTOR.upper(), center_b))
story.append(Spacer(1, 7.5 * cm))
story.append(Paragraph('<b>%s</b>' % TITULO, ParagraphStyle('t', parent=center_b, fontSize=16, leading=22)))
story.append(Spacer(1, 6))
story.append(Paragraph('<b>%s</b>' % (SUBTITULO[0].upper() + SUBTITULO[1:]), ParagraphStyle('st', parent=center, fontSize=13, leading=18)))
story.append(Spacer(1, 1 * cm))
story.append(Paragraph('Documentação técnica e manual do usuário — versão %s' % VERSAO, center))
story.append(Spacer(1, 10.2 * cm))
story.append(Paragraph(LOCAL, center))
story.append(Paragraph(ANO, center))
story.append(PageBreak())

# ------------------------------------------------------------------ FOLHA DE ROSTO
story.append(Paragraph(AUTOR.upper(), center_b))
story.append(Spacer(1, 7.5 * cm))
story.append(Paragraph('<b>%s</b>' % TITULO, ParagraphStyle('t2', parent=center_b, fontSize=16, leading=22)))
story.append(Spacer(1, 6))
story.append(Paragraph('<b>%s</b>' % (SUBTITULO[0].upper() + SUBTITULO[1:]), ParagraphStyle('st2', parent=center, fontSize=13, leading=18)))
story.append(Spacer(1, 2 * cm))
story.append(Paragraph('Documentação técnica do projeto Perícia Sim, versão %s, reunindo análise técnica, decisões de arquitetura, design da simulação, '
                       'manual do usuário, verificação, desempenho, limitações e matriz de rastreabilidade entre os Procedimentos '
                       'Operacionais Padrão de Informática Forense (SENASP/MJSP, 2024) e o comportamento do software.' % VERSAO, nature))
story.append(Spacer(1, 6.0 * cm))
story.append(Paragraph(LOCAL, center))
story.append(Paragraph(ANO, center))
story.append(PageBreak())

# ------------------------------------------------------------------ RESUMO
story.append(Paragraph('RESUMO', h1u))
P('Este documento descreve o Perícia Sim, simulador de treinamento em informática forense no estilo <i>hacking simulator</i>, '
  'construído a partir dos Procedimentos Operacionais Padrão (POP) 5.01 a 5.05 do volume 5 da coleção Perícia Criminal da '
  'Secretaria Nacional de Segurança Pública. O software, escrito em Rust, combina um ambiente tridimensional em primeira '
  'pessoa — laboratório pericial ou local de busca, com iluminação por pixel, sombras em tempo real e texturas procedurais — '
  'a uma estação pericial que executa um Linux simulado, com terminal compatível com o bash, gerenciador de arquivos, editor '
  'e aplicativos periciais, e a uma representação funcional da tela de aparelhos celulares. Cada caso é gerado de forma '
  'reprodutível a partir de uma semente, contém uma expressão de interesse escondida nos vestígios e exige que o perito '
  'siga o procedimento, preserve a evidência e documente o trabalho em laudo ou termo de apreensão exportável em PDF. '
  'Todas as ações são registradas em auditoria encadeada por SHA-256 e avaliadas por critérios explicados; perda de '
  'evidência limita a nota a 60%. A rede do jogo é emulada e isolada da internet real. O documento apresenta a análise '
  'dos materiais, a arquitetura, o método de design (ISAGA), o manual de uso, as 87 verificações automatizadas, as '
  'medições de desempenho sem placa de vídeo, as limitações conhecidas e a matriz de rastreabilidade entre os POPs e o software.')
story.append(Spacer(1, LEAD))
P('<b>Palavras-chave:</b> informática forense; simulação; treinamento pericial; cadeia de custódia; Rust.', body_ni)
story.append(PageBreak())

# ------------------------------------------------------------------ LISTAS E SUMÁRIO
lf = ListOf('FigEntry')
lq = ListOf('QEntry')
toc = TableOfContents()
lvl0 = ParagraphStyle('t0', fontName='Serif-B', fontSize=12, leading=LEAD, leftIndent=0, firstLineIndent=0)
lvl1 = ParagraphStyle('t1', fontName='Serif', fontSize=12, leading=LEAD, leftIndent=0.6 * cm, firstLineIndent=0)
lvl2 = ParagraphStyle('t2x', fontName='Serif', fontSize=12, leading=LEAD, leftIndent=1.2 * cm, firstLineIndent=0)
toc.levelStyles = [lvl0, lvl1, lvl2]
lst = ParagraphStyle('lst', fontName='Serif', fontSize=12, leading=LEAD, leftIndent=0, firstLineIndent=0)
lf.levelStyles = [lst]
lq.levelStyles = [lst]
toc.dotsMinLevel = 0
lf.dotsMinLevel = 0
lq.dotsMinLevel = 0
story.append(Paragraph('LISTA DE FIGURAS', h1u))
story.append(lf)
story.append(PageBreak())
story.append(Paragraph('LISTA DE QUADROS', h1u))
story.append(lq)
story.append(PageBreak())
story.append(Paragraph('LISTA DE ABREVIATURAS E SIGLAS', h1u))
sig = [('ABNT', 'Associação Brasileira de Normas Técnicas'), ('ADR', 'Architecture Decision Record (registro de decisão de arquitetura)'),
       ('AFU', 'After First Unlock (após o primeiro desbloqueio)'), ('BFU', 'Before First Unlock (antes do primeiro desbloqueio)'),
       ('CGNAT', 'Carrier-Grade NAT (tradução de endereços em escala de operadora)'), ('DKIM', 'DomainKeys Identified Mail'),
       ('DNS', 'Domain Name System'), ('EXIF', 'Exchangeable Image File Format'), ('FBE', 'File-Based Encryption'),
       ('FDE', 'Full-Disk Encryption'), ('GLSL', 'OpenGL Shading Language'), ('ICCID', 'Integrated Circuit Card Identifier'),
       ('IMEI', 'International Mobile Equipment Identity'), ('ISAGA', 'International Simulation and Gaming Association'),
       ('MJSP', 'Ministério da Justiça e Segurança Pública'), ('PCF', 'Percentage-Closer Filtering'),
       ('POP', 'Procedimento Operacional Padrão'), ('RF', 'Radiofrequência'), ('SENASP', 'Secretaria Nacional de Segurança Pública'),
       ('SHA-256', 'Secure Hash Algorithm, 256 bits'), ('SIM', 'Subscriber Identity Module'), ('SPF', 'Sender Policy Framework')]
t = Table([[Paragraph(a, body_ni), Paragraph(b, body_ni)] for a, b in sig], colWidths=[3 * cm, TEXT_W - 3 * cm])
t.setStyle(TableStyle([('VALIGN', (0, 0), (-1, -1), 'TOP'), ('LEFTPADDING', (0, 0), (-1, -1), 0)]))
story.append(t)
story.append(PageBreak())
story.append(Paragraph('SUMÁRIO', h1u))
story.append(toc)
story.append(PageBreak())

# ------------------------------------------------------------------ 1 INTRODUÇÃO
story.append(Marker())
story.append(Paragraph('1 INTRODUÇÃO', h1))
P('A perícia de informática é feita de procedimentos sequenciais e, em boa parte, irreversíveis: ligar um equipamento '
  'apreendido sem bloqueador de escrita altera a mídia original; desligar um aparelho celular no estado AFU descarta as '
  'chaves de criptografia mantidas em memória; navegar em um site investigado a partir da rede institucional pode alertar '
  'o investigado. Errar num caso real destrói prova. Um simulador permite errar, observar a consequência e refletir sobre ela.')
P('O Perícia Sim é um simulador de treinamento que transforma os Procedimentos Operacionais Padrão de Informática Forense '
  '(BRASIL, 2024) em regras de jogo. O perito em formação recebe um caso gerado proceduralmente, percorre o ambiente em '
  'primeira pessoa, manipula os vestígios na bancada, adquire as mídias, examina as cópias numa estação pericial que '
  'executa um Linux simulado e, ao final, emite o laudo (POPs 5.01 a 5.04) ou o termo de apreensão (POP 5.05). '
  'Cada caso tem uma <b>expressão de interesse</b> escondida nos vestígios; encontrá-la não basta, porque a avaliação '
  'considera procedimento, preservação, rastreabilidade e fundamentação.')
P('Este documento reúne a documentação atualizada da versão %s: análise técnica dos materiais, arquitetura e decisões '
  'tecnológicas, design da simulação, descrição funcional com capturas de tela, manual de uso, verificação, desempenho, '
  'limitações e, no Apêndice A, a matriz de rastreabilidade entre os POPs e o comportamento do software.' % VERSAO)
P('<b>Aviso.</b> Todo o conteúdo dos casos é fictício. A “internet” do jogo é emulada dentro do programa; nenhuma ferramenta '
  'investigativa acessa a rede real e o terminal não executa comandos do sistema operacional hospedeiro. Recuperação de '
  'dados, extração móvel, memória volátil e rede são modelos didáticos coerentes com o estado do caso, mas não reproduzem '
  'ferramentas reais. Nada aqui é documento oficial.')

# ------------------------------------------------------------------ 2 MATERIAIS
story.append(PageBreak())
story.append(Paragraph('2 MATERIAIS E REQUISITOS', h1))
story.append(Paragraph('2.1 MATERIAIS ANALISADOS', h2))
P('O projeto partiu da leitura integral dos materiais indicados pelo solicitante, resumidos no Quadro 1.')
quadro('Materiais analisados', ['Material', 'Uso no projeto'], [
    ['Perícia Criminal 2024 — Informática Forense, vol. 5 (SENASP/MJSP), 69 p.', 'Fonte das regras: POPs 5.01 a 5.05, glossários e referências.'],
    ['Escopo do simulador de informática forense (ABNT), 19 p.', 'Escopo, módulos, rede emulada e critérios de aceitação.'],
    ['Prompt mestre do solicitante', 'Requisitos adicionais e regras de trabalho.'],
    ['Q99 Studio — How to build a simulation game', 'Conceito, sistemas inter-relacionados, laço central, protótipo e iteração.'],
    ['Summer Engine — How to make a simulation game with AI', 'Um laço mínimo por vez; validar antes de adicionar; ajuste numérico.'],
    ['Peters e van de Westelaken (2014), ISAGA', 'Método de design: 4 fases/10 passos, matriz componentes × elementos, debriefing.'],
], [0.5, 0.5])

story.append(Paragraph('2.2 REQUISITOS EXTRAÍDOS POR POP', h2))
reqs = [
    ('POP 5.01 — Mídias de armazenamento.', 'Atenção a vestígios físicos; conferir descrição e lacre antes do deslacre; fotografar lacrado; '
     'remover a mídia e registrar a interface; exame sobre a cópia; bloqueador de escrita; verificação de hash; criptografia dependente '
     'do equipamento exige espelhamento com destino sanitizado; processamento (recuperação, expansão de compostos, OCR, detecção de '
     'criptografia); tratamento de senhas por dicionário; laudo com material, original × cópia, softwares e novo lacre. Pontos críticos: '
     'RAID/JBOD, ordem de boot, nuvem só com autorização judicial, fuso horário.'),
    ('POP 5.02 — Dispositivos móveis.', 'Isolamento de radiofrequência; remover SIM registrando o slot; manter em AFU isolado e carregando; '
     'modo avião e verificação das conexões; desbloqueio com risco de restauração de fábrica; desabilitar alarmes; extração mais completa '
     'possível; verificar a integridade da extração antes de desligar; IMEI físico × lógico; segundo espaço e cofres; laudo com IMEIs, ICCID e operadora.'),
    ('POP 5.03 — Local de informática.', 'Levantamento prévio e restrições legais; reconhecer, isolar e fixar; equipamento ligado: ordem de '
     'volatilidade, dump de memória quando houver interesse, copiar partições cifradas montadas; não iniciar o sistema instalado; mídias '
     'ocultas (SSD sob dissipador); hashes e lacres no termo; embalagem antiestática.'),
    ('POP 5.04 — Local de internet.', 'Conexão desvinculada da instituição; WHOIS, traceroute, DNS; atribuição depende do provedor e da '
     'porta lógica em CGNAT; e-mail: cabeçalho completo, campos Received em ordem inversa, DKIM, SPF; preservar páginas no formato '
     'original registrando data, hora e IP usado; nuvem preferencialmente por quebra judicial.'),
    ('POP 5.05 — Apreensão de móveis.', 'Baterias externas; reconhecer, isolar e fixar; etiqueta na câmera frontal; solicitar senhas e '
     'verificar smartlock; modo avião e registro se foi possível; desabilitar alarmes; remover SIM registrando slot, operadora e ICCID; '
     'AFU em Faraday bag ou três camadas de alumínio com bateria externa; embalar e lacrar individualmente. Estrutura de laudo: não se '
     'aplica — o documento é o termo de apreensão.'),
]
for a, b in reqs:
    P('<b>%s</b> %s' % (a, b))

story.append(Paragraph('2.3 AMBIGUIDADES E DECISÕES', h2))
bl([
    'O POP 5.03 tem duas seções “Ações Preliminares”; interpretou-se 4.1 como preparatórias e 4.2 como ações no local.',
    'A estrutura de laudo aparece nos POPs 5.01 a 5.04; o 5.05 diz “não se aplica”. O simulador emite termo de apreensão no 5.05.',
    'Os campos do termo de apreensão foram derivados do que o POP manda “constar no termo” (decisão de projeto).',
    'O algoritmo de hash não é nomeado no POP (“criptograficamente seguro”); adotou-se SHA-256.',
    'O POP 5.03 menciona bases de hashes de exploração sexual infantil; o simulador não gera nem descreve esse conteúdo e usa, '
    'em seu lugar, bases de hashes de documentos sigilosos fictícios.',
    'Recuperação, extração móvel e memória volátil são modelos; os resultados são coerentes com o estado do caso.',
])

# ------------------------------------------------------------------ 3 ARQUITETURA
story.append(PageBreak())
story.append(Paragraph('3 ARQUITETURA E TECNOLOGIA', h1))
story.append(Paragraph('3.1 VISÃO GERAL', h2))
P('O software é um <i>workspace</i> Rust com dois módulos (<i>crates</i>). O motor, ' + c('pericia_core') + ', é Rust puro, '
  'depende apenas de serde e serde_json e não contém nenhuma API de rede nem de execução de processos — um teste automatizado '
  'verifica o código-fonte e a árvore de dependências. A interface, ' + c('pericia_app') + ', usa a biblioteca macroquad e '
  'conversa com o motor exclusivamente pelo método ' + c('Sim::exec(comando)') + ', que devolve o texto da resposta. '
  'Assim, cada botão da interface corresponde a um comando de terminal, e o jogador aprende a sintaxe enquanto joga.')
quadro('Módulos do motor (pericia_core)', ['Módulo', 'Responsabilidade'], [
    ['casegen, casegen_disk, casegen_mobile, casegen_scene, casegen_net', 'Geração procedural reprodutível dos cinco módulos POP, quatro dificuldades.'],
    ['vfs, osapi', 'Sistema virtual de evidências: partições, arquivos apagados, compostos, cifrados, setores ruins, metadados, hash.'],
    ['engine', 'Regras dos POPs, tempo simulado, eventos adversos, bateria, faltas.'],
    ['tools', 'Comandos de exame e da rede emulada (DNS, WHOIS, navegação, e-mail, CGNAT).'],
    ['phone', 'Tela do aparelho celular (estados, aplicativos) e regras da navegação manual.'],
    ['os (mod, shell, cmds, man)', 'Linux simulado: árvore de arquivos, montagens, shell compatível com bash, utilitários, páginas de manual.'],
    ['audit, sha256', 'Auditoria encadeada por SHA-256 e implementação própria do algoritmo.'],
    ['report, pdf', 'Laudo/termo, coerência com a auditoria, exportação em Markdown e PDF (gerador próprio).'],
    ['assess, manual', 'Avaliação explicada, mentor adaptativo, resumos dos POPs e matriz de rastreabilidade.'],
], [0.42, 0.58])
quadro('Módulos da interface (pericia_app)', ['Módulo', 'Responsabilidade'], [
    ['main', 'Laço principal, menu, área de hardware, painel de itens, roteiros de captura automatizada.'],
    ['render', 'Renderizador próprio: texturas procedurais, shader GLSL, luzes, mapa de sombras.'],
    ['world', 'Construção da sala, mobília, itens do caso, colisores e pontos de interação.'],
    ['fps', 'Controle em primeira pessoa: movimento, olhar, colisão, balanço da cabeça.'],
    ['linuxde', 'Ambiente de desktop do Linux simulado: janelas, painel, terminal, arquivos, editor.'],
    ['desktop', 'Aplicativos periciais (navegador, evidências, documento, manual) reutilizados nas janelas.'],
    ['phone', 'Desenho e interação da tela do aparelho celular.'],
    ['save, ui', 'Persistência por replay, configurações e kit de interface imediata.'],
], [0.25, 0.75])

story.append(Paragraph('3.2 DECISÃO TECNOLÓGICA', h2))
P('A especificação sugeria Rust com egui (2D); o solicitante pediu modelos 3D e uma linguagem compilada, não TypeScript/HTML. '
  'O Quadro 4 resume as opções avaliadas no ADR-0001.')
quadro('Opções tecnológicas avaliadas (ADR-0001)', ['Opção', '3D', 'Requisito de GPU', 'Observação'], [
    ['Rust + egui/eframe', 'sem cena 3D pronta', 'GL 3 / wgpu', 'não atende ao requisito de 3D'],
    ['Rust + Bevy', 'completo', 'Vulkan / GL ES 3', 'binário 30–80 MB; GPUs antigas podem falhar'],
    ['C++ + raylib', 'bom', 'GL 3.3 (ou 2.1)', 'memória manual, menos testável'],
    ['<b>Rust + macroquad (escolhida)</b>', 'bom, com malhas próprias', 'OpenGL 2.1 / GLES 2', 'binário de ~4 MB; roda com Mesa por software'],
], [0.3, 0.2, 0.22, 0.28])
P('Na versão 0.3, a iluminação deixou de ser calculada por face na CPU e passou a um shader GLSL 100 próprio, ainda compatível '
  'com OpenGL 2.1/GLES 2: três luzes pontuais, sol, ambiente hemisférico, especular Blinn-Phong, névoa e mapeamento de tons, '
  'com mapa de sombras de 1024² e filtragem PCF de cinco amostras.')

story.append(Paragraph('3.3 PERSISTÊNCIA POR REPLAY (ADR-0002)', h2))
P('Toda interação — inclusive as linhas digitadas no terminal do Linux simulado e as gravações do editor — passa pelo motor. '
  'O salvamento guarda apenas o identificador do caso, a versão do gerador e a lista de comandos. Carregar significa gerar o '
  'caso pela semente e reexecutar os comandos, conferindo em seguida a cadeia de auditoria. Isso exige determinismo: nenhum uso '
  'de relógio real nem de aleatoriedade fora da semente. Como consequência positiva, um instrutor reproduz exatamente a sessão '
  'de um aluno a partir do arquivo de salvamento.')

story.append(Paragraph('3.4 ISOLAMENTO', h2))
P('O isolamento em relação à rede real é estrutural: o motor não depende de nenhuma biblioteca de rede, e a rede do caso '
  '(domínios, DNS, WHOIS, páginas, e-mails, registros de provedor) é um modelo de dados gerado com o caso. O terminal do Linux '
  'simulado interpreta os comandos dentro do programa; nada é repassado ao sistema operacional hospedeiro.')

# ------------------------------------------------------------------ 4 DESIGN
story.append(PageBreak())
story.append(Paragraph('4 DESIGN DA SIMULAÇÃO', h1))
P('O design seguiu o método da ISAGA (PETERS; VAN DE WESTELAKEN, 2014): especificação, análise do sistema de referência, '
  'desenho do jogo e construção com testes, complementado pelas recomendações de iteração curta de Q99 Studio e Summer Engine.')
story.append(Paragraph('4.1 ESPECIFICAÇÃO', h2))
quadro('Especificação do jogo', ['Pergunta', 'Decisão'], [
    ['Por que simular?', 'Procedimentos periciais são sequenciais e irreversíveis; a simulação permite errar e ver a consequência.'],
    ['Objetivo de aprendizagem', 'Executar os POPs na ordem correta, preservar a evidência, interpretar resultados com seus limites e documentar de forma coerente.'],
    ['Público', 'Peritos em formação e estudantes de computação forense; uso individual.'],
    ['Tom', 'Intelectual (decisão e informação). Sem cronômetro real; há tempo simulado que avança a cada ação.'],
    ['Abstração', 'Média: equipamentos e dados são modelos; procedimentos são fiéis ao POP.'],
    ['Avaliação', 'Indicadores explícitos e explicados ao final.'],
], [0.28, 0.72])
story.append(Paragraph('4.2 ANÁLISE DO SISTEMA DE REFERÊNCIA', h2))
P('Atores: o perito (papel jogado); a autoridade requisitante, o investigado, o provedor e o proprietário do aparelho (papéis '
  'simulados). Objetos: itens físicos, mídias e partições, arquivos ativos, apagados, compostos e cifrados, memória volátil, rede e '
  'documentos. Relações críticas: lacre e documentação; original e cópia (hash); equipamento ligado e dados voláteis; conectividade '
  'e risco de comando remoto; IP, assinante e pessoa (que não são equivalentes); relatório e auditoria.')
story.append(Paragraph('4.3 MATRIZ COMPONENTES × ELEMENTOS DE JOGO', h2))
quadro('Componentes do sistema × elementos de jogo', ['Componente', 'Regras', 'Eventos', 'Indicadores'], [
    ['Recebimento e lacre', 'Conferir antes de deslacrar', 'Divergência de lacre (D≥2)', 'Conformidade; custódia'],
    ['Preservação/aquisição', 'Exame só sobre cópia; bloqueador', '—', 'Preservação; integridade'],
    ['Análise de arquivos', 'Apagado → recuperar; composto → expandir; cifrado → decifrar', 'Setores ruins, sobrescrita parcial', 'Uso de ferramentas; interpretação'],
    ['Dados voláteis', 'Coletar memória/volume antes de desligar', 'Apagamento remoto se conectado (D≥3)', 'Preservação'],
    ['Dispositivo móvel', 'Isolar, SIM, alimentação, extração; navegação manual só documentada e após a extração', 'Comando remoto; bateria esgotada; limite de tentativas', 'Preservação; conformidade'],
    ['Rede', 'Conexão desvinculada; preservar; correlacionar', 'Retirada da página se alertado', 'Correlação; limitações'],
    ['Documento', 'Estrutura por POP; 5.05 = termo', '—', 'Qualidade documental; coerência'],
], [0.2, 0.34, 0.24, 0.22], small=True)
story.append(Paragraph('4.4 FORMATO, CONTABILIZAÇÃO E DEBRIEFING', h2))
P('O macrociclo é: menu, briefing (caso, quesitos, autorização), rodadas de ação, documento, finalização, avaliação e histórico. '
  'No microciclo, cada ação aplica a regra do POP, avança o tempo simulado, grava a auditoria e devolve o resultado. Há quatro '
  'dificuldades: 1 Guiado (mentor com checklist e justificativa), 2 Assistido (próximo passo sob demanda), 3 Autônomo (só contagem '
  'de etapas) e 4 Avançado (sem orientação, com eventos adversos e evidências distribuídas).')
quadro('Critérios de avaliação', ['Critério', 'Pontos'], [
    ['Conformidade com o procedimento (POP)', '20'], ['Preservação das evidências e integridade', '20'],
    ['Registro de ações e cadeia de custódia', '10'], ['Uso adequado das ferramentas e interpretação', '10'],
    ['Correlação entre fontes', '10'], ['Distinção fato/inferência e reconhecimento de limitações', '10'],
    ['Qualidade documental', '10'], ['Expressão de interesse fundamentada (não se aplica ao 5.05)', '10'],
    ['<b>Total</b> (90 na apreensão). Perda ou destruição de evidência limita a nota a 60%.', '<b>100</b>'],
], [0.82, 0.18])
P('Cada ponto perdido vem acompanhado da frase que o explica. O laudo exportado inclui “Procedimentos não realizados”, e o '
  'histórico permite repetir o mesmo caso pelo identificador.')

# ------------------------------------------------------------------ 5 DESCRIÇÃO FUNCIONAL
story.append(PageBreak())
story.append(Paragraph('5 DESCRIÇÃO FUNCIONAL', h1))
P('Esta seção apresenta o simulador como o usuário o vê. As capturas foram feitas na versão %s, em resolução de 1280 × 760, '
  'com renderização OpenGL por software (Mesa llvmpipe), sem placa de vídeo — o pior caso realista.' % VERSAO)
story.append(Paragraph('5.1 MENU E CASOS REPRODUTÍVEIS', h2))
P('No menu escolhem-se o módulo (POP 5.01 a 5.05), a dificuldade e, opcionalmente, a semente ou o identificador de um caso '
  '(por exemplo, ' + c('C501-D2-00000007') + '). O mesmo identificador gera sempre o mesmo caso. As configurações permitem '
  'desligar a cena 3D (modo 2D leve), a economia de energia e as sombras, e ajustar a escala da interface.')
figure(f'{D}/f01_menu.png', 'Menu principal com módulos, dificuldades e configurações')

story.append(Paragraph('5.2 AMBIENTE EM PRIMEIRA PESSOA', h2))
P('O ambiente é percorrido a pé, sem personagem visível. As teclas W, A, S e D movem o perito (Shift corre); o botão direito do '
  'mouse, ou a tecla Tab, controla o olhar. Há colisão com paredes e mobília. Mirando um item a até 2,6 m, um clique ou a tecla E '
  'abre o painel com o estado do item e as ações físicas permitidas — cada uma é um comando do terminal. Os módulos 5.01, 5.02 e '
  '5.04 ocorrem no laboratório; os módulos 5.03 e 5.05, no local de busca (escritório).')
figure(f'{D}/f02_laboratorio.png', 'Laboratório pericial: bancada antiestática, item lacrado e estação pericial ao fundo')
figure(f'{D}/f03_bancada_painel.png', 'Aproximação da bancada com o item selecionado e o painel de ações físicas')
figure(f'{D}/f04_escritorio.png', 'Local de busca (POP 5.03): escritório com dois computadores, estante e mobília')
story.append(Paragraph('5.3 GRÁFICOS', h2))
P('Os modelos são gerados por código (<i>low-poly</i>) e recebem texturas procedurais de 256 × 256 pixels com <i>mipmaps</i>: '
  'madeira com veio longitudinal, piso vinílico e de madeira, parede, forro, metal escovado, plástico, tecido, manta antiestática, '
  'grade de ventilação, teclado, etiquetas, tapete, folhagem, papelão, concreto, vidro e a vista externa pela janela (céu com nuvens, '
  'prédios e árvores). O shader calcula a iluminação por pixel e aplica sombras a partir das luminárias do teto. O monitor da '
  'estação, na cena 3D, mostra uma captura real da tela do Linux simulado.')
figure(f'{D}/f05_janela.png', 'Iluminação por pixel, sombras projetadas no piso e vista externa pela janela')
figure(f'{D}/f09_monitor_3d.png', 'Estação pericial na cena 3D, com o monitor exibindo a sessão do Linux simulado')

story.append(Paragraph('5.4 ESTAÇÃO PERICIAL: LINUX SIMULADO', h2))
P('Ao sentar na estação, o perito entra direto (sem senha) no Perícia Linux 12.4 “Lacre”, um Linux simulado — não é máquina '
  'virtual. O ambiente tem painel com menu Aplicativos e lista de janelas, janelas móveis e redimensionáveis, terminal, '
  'gerenciador de arquivos, editor de texto e os aplicativos periciais. O shell aceita <i>pipes</i>, redirecionamento, ' + c('&&') + ' e '
  + c('||') + ', variáveis, curingas (inclusive no meio do caminho), histórico e completação com Tab. Há ' + c('/dev') + ', '
  + c('/proc') + ', ' + c('/mnt') + ' e montagens somente leitura de imagens forenses. Montar o dispositivo original é registrado '
  'como falta e, sem bloqueador de escrita, contamina a evidência. As operações periciais são feitas com o comando '
  + c('pericia <comando>') + ', que passa pelas regras do POP e pela auditoria.')
figure(f'{D}/f06_terminal.png', 'Terminal: aquisição da imagem, montagem somente leitura e busca pela expressão')
figure(f'{D}/f07_arquivos.png', 'Gerenciador de arquivos navegando na imagem montada')
figure(f'{D}/f08_menu_apps.png', 'Menu Aplicativos com as ferramentas do sistema e da perícia')

story.append(Paragraph('5.5 APLICATIVOS PERICIAIS', h2))
P('O navegador acessa apenas a rede emulada do caso e permite escolher a conexão (institucional, dedicada ou anônima), '
  'consultar DNS, WHOIS, traceroute, registros de provedor e cabeçalhos de e-mail, e preservar páginas. A linha do tempo '
  'reúne os eventos em horário de Brasília, convertendo o fuso das partições. O documento pericial organiza o laudo pelos '
  'campos do POP, com pré-preenchimento a partir da auditoria, verificação de coerência e exportação em PDF e Markdown. '
  'A avaliação explica cada critério, e a cadeia de custódia mostra os artefatos e a auditoria encadeada.')
figure(f'{D}/f10_navegador.png', 'Navegador da rede emulada (POP 5.04) com conexão anônima selecionada')
figure(f'{D}/f12_documento.png', 'Documento pericial com campos do laudo e pré-preenchimento')
figure(f'{D}/f13_avaliacao.png', 'Avaliação explicada ao final do caso')
figure(f'{D}/f19_custodia.png', 'Cadeia de custódia com auditoria encadeada por SHA-256')

story.append(Paragraph('5.6 TELA DO APARELHO CELULAR', h2))
P('Selecionando um celular e escolhendo “Ver a tela do aparelho”, o perito segura o aparelho diante da cena. A tela reflete o '
  'estado do motor: apagada quando desligado ou sem carga; bloqueada antes do primeiro desbloqueio (BFU), sem notificações; '
  'bloqueada após o primeiro desbloqueio (AFU), com a contagem de notificações e o conteúdo oculto; assistente de configuração '
  'quando o aparelho foi restaurado de fábrica; e tela inicial quando desbloqueado. A barra de status mostra hora, bateria, '
  'modo avião e sinal; tela trincada e etiqueta sobre a câmera frontal aparecem quando presentes no caso.')
P('Na tela inicial há Telefone (registro de chamadas e código *#06#), Mensagens (conversas do mensageiro e SMS do chip), '
  'Contatos, Galeria (fotos com metadados EXIF, inclusive GPS), Arquivos (itens apagados não aparecem), Relógio (alarmes), '
  'Configurações (modo avião, segurança, chips, sobre o telefone) e, quando existe, o Espaço seguro protegido por senha própria. '
  'Com a tela bloqueada, o botão Emergência dá acesso às informações de emergência e ao código de serviço *#06#, que confere o '
  'IMEI lógico com a etiqueta física. A tela não oferece meio de desbloqueio: o acesso depende do estado do aparelho, da senha '
  'documentada ou da extração.')
figure(f'{D}/c_cel_desbloqueado.png', 'Aparelho desbloqueado: tela inicial, conversa no mensageiro e foto com metadados', w=TEXT_W)
figure(f'{D}/c_cel_bloqueado.png', 'Aparelho bloqueado em BFU com tela trincada e acesso de emergência ao código *#06#', w=TEXT_W * 0.66)
P('Cada aplicativo aberto vira o comando ' + c('tela <item> <app>') + ', registrado na auditoria, e as regras dos POPs se aplicam: '
  'no exame (5.02), navegar no conteúdo antes da extração é falta e marca as mensagens como lidas; depois da extração, a '
  'navegação é complemento documentado com ' + c('tela <item> foto') + '. Na apreensão (5.05), abrir conteúdo é falta, enquanto '
  'configurações, alarmes e IMEI são permitidos. Se a expressão de interesse for vista na tela, ela é registrada com a origem.')

story.append(Paragraph('5.7 LAUDO EXPORTADO', h2))
P('O laudo e o termo são exportados por um gerador de PDF próprio do motor, com destaque da expressão de interesse e ligação '
  'com a origem, e também em Markdown. O estilo é inspirado em convenções ABNT, sem alegar conformidade normativa.')
figure(f'{D}/c_laudo.png', 'Primeiras páginas de um laudo de exemplo exportado pelo simulador', w=TEXT_W * 0.9)


# ------------------------------------------------------------------ 6 INTEGRIDADE (v0.4)
story.append(PageBreak())
story.append(Paragraph('6 INTEGRIDADE, COMPLETUDE E RECURSOS EXTERNOS (VERSÃO 0.4.0)', h1))
P('A versão 0.4.0 foi uma evolução incremental: primeiro se mediu o estado atual (60 testes passando, nenhum aviso de compilação), '
  'depois se procuraram falhas reais — inclusive com 500 sequências aleatórias de comandos em todos os módulos e 60 sessões de shell '
  'com entradas malformadas, sem nenhum pânico — e só então se completaram as limitações. A arquitetura, a interface e as mecânicas '
  'existentes foram preservadas.')
story.append(Paragraph('6.1 FALHAS ENCONTRADAS E CORRIGIDAS', h2))
quadro('Falhas reais encontradas na auditoria da versão 0.3.1', ['Falha', 'Consequência', 'Correção'], [
    ['Cadeia de auditoria juntava os campos com "|"', 'Linha de shell com pipe podia gerar o mesmo conteúdo assinado de outra entrada', 'Formato 2: campos prefixados pelo tamanho; salvamentos antigos conferidos pelo hash legado'],
    ['Salvamento gravava o último hash mas não o conferia', 'Divergência de reprodução passava despercebida', 'Trilha da auditoria gravada; ao carregar, aviso aponta a primeira entrada divergente'],
    ['Gravação direta do arquivo de salvamento', 'Queda no meio da gravação corrompia o caso', 'Gravação atômica (temporário + renomear)'],
    ['Comando remoto de apagamento descartado se o aparelho estivesse isolado no instante', 'Religar fora da Faraday não tinha consequência', 'Comando fica pendente até haver conectividade'],
    ['"sanitizar m-01" marcava a evidência como zerada; "clone" aceitava destino vazio ou a própria mídia e não exigia bloqueador', 'Ação destrutiva sem consequência pedagógica', 'Recusa, registro na auditoria e falta de procedimento'],
    ['Termo de apreensão imprimia a senha do aparelho mesmo sem ter sido informada', 'Documento afirmava um fato não verificado', 'Só consta senha efetivamente informada'],
    ['Documentação afirmava "bateria não se esgota" e "Procedimentos não realizados" no Markdown', 'Documentação sem comprovação', 'Matriz corrigida; seção adicionada ao Markdown e ao texto do laudo'],
], [0.34, 0.33, 0.33], small=True)
P('As mensagens de erro passaram a dizer o que faltou e quais itens ou mídias existem — sem revelar mídia ainda oculta — e a sugerir o '
  'comando correto em caso de erro de digitação (por exemplo, “imagen” → “imagem”).')
story.append(Paragraph('6.2 LIMITAÇÕES COMPLETADAS', h2))
P('<b>Evidências em nuvem (POP 5.01, pontos críticos; POP 5.04).</b> O comando ' + c('nuvem') + ' mostra o alcance da decisão judicial. '
  'Com autorização, ' + c('nuvem solicitar <conta>') + ' encaminha a requisição ao provedor (caminho preferencial; os dados chegam após um prazo, '
  'com hash), e ' + c('nuvem acessar <conta> <senha>') + ' faz a captura direta, registrando IP e horário — três senhas erradas bloqueiam a conta. '
  'Sem autorização, a requisição não é encaminhada, a tentativa de captura direta é falta e a limitação é escrita automaticamente no laudo.')
figure(f'{D}/02_nuvem_sem_autorizacao.png', 'Nuvem sem autorização judicial: o acesso é recusado e a tentativa registrada')
figure(f'{D}/03_nuvem_provedor.png', 'Nuvem com autorização: requisição ao provedor e listagem dos dados entregues')
P('<b>Cifragem integral por hardware e ordem de inicialização.</b> O gerador 1.1.0 sorteia, no POP 5.01 dificuldade 3, discos '
  'autocriptografados vinculados ao equipamento. A imagem isolada é recusada; o caminho do POP é sanitizar a mídia de destino, espelhar, '
  'conferir a ordem de inicialização com ' + c('bios') + ' e desbloquear o clone no equipamento de origem com a senha documentada. Operar o '
  'equipamento sem conferir a BIOS faz o sistema instalado iniciar e é registrado como falta.')
figure(f'{D}/01_cifragem_hardware.png', 'Disco cifrado por hardware: imagem recusada, sanitização da evidência recusada, espelhamento e desbloqueio do clone')
P('<b>Arquivos danificados e cadeia ARC.</b> Nos POPs 5.01 e 5.03 (dificuldade 2 ou mais), cada caso passa a ter um arquivo apagado e '
  'sobrescrito (só o registro de diretório sobrevive), um arquivo em setores ilegíveis e um PDF com extensão .jpg, cuja assinatura aparece no '
  'comando ' + c('file') + '. No POP 5.04 (dificuldade 3 ou mais), a mensagem pode ter passado por um intermediário que quebrou o DKIM; a cadeia '
  'ARC registra a autenticação feita no meio do caminho, e ' + c('email cabecalho') + ' explica a leitura.')
P('<b>Compatibilidade de casos.</b> As versões do gerador passaram a ser acumulativas (ADR-0003): o conteúdo-base de cada ID não muda e a '
  'versão 1.1.0 só acrescenta extensões. Salvamentos da versão 0.3 continuam carregando e reproduzindo o mesmo caso.')
P('<b>Laudo: fatos, hipóteses e limitações.</b> O laudo passou a ter as seções Resultados — fatos observados (cada fato com a sua fonte), '
  'Considerações técnicas e limitações, Hipóteses não confirmadas (campo próprio no editor) e Procedimentos previstos não realizados. A '
  'verificação de coerência aponta linguagem de hipótese na conclusão (“provavelmente”, “pode ter”) e conclusões que afirmam achados sem '
  'nenhum fato registrado.')
figure(f'{D}/04_documento_hipoteses.png', 'Editor do laudo com o campo de hipóteses separado da conclusão')
P('<b>Editor.</b> O editor de texto do Linux simulado e os campos do documento passaram a ter cursor real: setas, Home/End, Delete e clique '
  'para posicionar. A quebra de linha foi reescrita com custo linear, para não pesar no quadro.')
story.append(Paragraph('6.3 PREPARAÇÃO PARA MODELOS 3D E TEXTURAS FORNECIDOS', h2))
P('Os modelos e as texturas que serão fornecidos ainda não chegaram; nenhum asset foi inventado para substituí-los. O projeto foi '
  'preparado para recebê-los sem trocar a tecnologia gráfica: uma pasta ' + c('assets/') + ' opcional (sem ela, nada muda); substituição '
  'de qualquer textura procedural por um PNG/TGA de mesmo nome, com validação de potência de 2 e limite de 2048 px; leitura e validação de '
  'modelos OBJ/MTL (triangulação, materiais, caixa envolvente, texturas ausentes) com relatório gravado na pasta de dados do usuário — nunca '
  'na pasta dos originais; e uma ferramenta de inventário, somente leitura, que registra o SHA-256 de cada arquivo, as dimensões, as '
  'contagens de malha, as referências ausentes, a presença de licença e a compatibilidade (glTF/GLB, FBX e .blend exigem conversão ou '
  'leitor novo, decisão a tomar quando os arquivos forem avaliados).')
figure(f'{D}/f20_assets_teste.png', 'Teste do mecanismo de substituição com uma textura de teste no tapete (arquivo de teste, não distribuído)')
story.append(Paragraph('6.4 PLATAFORMAS', h2))
quadro('Plataformas e o que foi efetivamente verificado', ['Plataforma', 'Situação'], [
    ['Linux — Ubuntu 24.04 (glibc 2.39), ambiente de desenvolvimento', 'Compilado, 87 testes, execução e medições sob Xvfb + Mesa llvmpipe'],
    ['Linux — Ubuntu 22.04 (glibc 2.35), máquina do usuário', 'Binário executado sob Xvfb; ambiente, celular e estação renderizados (Figura a seguir)'],
    ['Linux com glibc anterior a 2.34 (ex.: Ubuntu 20.04)', 'Binário pronto não roda; compilar no próprio sistema'],
    ['Windows e macOS', 'Não compilados nem testados: o ambiente não tem acesso às ferramentas de compilação cruzada'],
], [0.45, 0.55])
figure(f'{D}/c_ubuntu2204.png', 'Execução no Ubuntu 22.04: ambiente, tela do celular e estação pericial')

# ------------------------------------------------------------------ 6 MANUAL
story.append(PageBreak())
story.append(Paragraph('7 MANUAL DE USO', h1))
story.append(Paragraph('7.1 INSTALAÇÃO', h2))
P('No Linux x86_64, descompacte o pacote e execute o programa, ou instale-o no menu de aplicativos sem privilégios de administrador:')
story.append(Paragraph(escape('tar xzf pericia-sim-0.4.0-linux-x86_64.tar.gz\ncd pericia-sim-0.4.0-linux-x86_64\n./pericia-sim        # executar\n./install.sh         # ou instalar no menu').replace('\n', '<br/>'), code))
P('São necessárias apenas as bibliotecas OpenGL/X11 do sistema; sem GPU, funciona com o driver por software do Mesa. Para compilar '
  'a partir do código (Linux, Windows ou macOS), instale o Rust e execute ' + c('cargo run --release -p pericia_app') + '. '
  'Os dados ficam em ' + c('~/.local/share/pericia-sim') + ' (Linux), ' + c('%APPDATA%\\pericia-sim') + ' (Windows) ou '
  + c('~/Library/Application Support/pericia-sim') + ' (macOS): caso em andamento, histórico e documentos exportados.')
story.append(Paragraph('7.2 CONTROLES', h2))
quadro('Controles', ['Contexto', 'Ação', 'Controle'], [
    ['Ambiente', 'Andar / correr', 'W, A, S, D ou setas / Shift'],
    ['Ambiente', 'Olhar', 'Botão direito pressionado, ou Tab para capturar o mouse (Esc solta)'],
    ['Ambiente', 'Interagir', 'Mirar e clicar, ou tecla E (alcance de 2,6 m)'],
    ['Ambiente', 'Lista de itens', 'Botão “Itens e ações”'],
    ['Estação', 'Sentar / levantar', 'Clicar no monitor / “Levantar da estação”'],
    ['Terminal', 'Histórico / completar', 'Setas para cima e para baixo / Tab'],
    ['Celular', 'Ver a tela / guardar', '“Ver a tela do aparelho” / Esc'],
    ['Celular', 'Voltar / início', 'Botões da barra de navegação do aparelho'],
], [0.2, 0.3, 0.5])
story.append(Paragraph('7.3 ROTEIRO DE UM CASO', h2))
for i, t in enumerate([
    'No menu, escolha o POP e a dificuldade (comece por 1 · Guiado) e clique em “Iniciar novo caso”.',
    'Leia o caso pelo Mentor ou pelo aplicativo Caso: histórico, objetivo, quesitos e autorização legal.',
    'No ambiente, aproxime-se dos itens e use as ações do painel na ordem do POP: fotografar, conferir o lacre, deslacrar, isolar, e assim por diante.',
    'Adquira a mídia (bloqueador, imagem, verificação de hash) ou extraia o celular.',
    'Na estação, examine a cópia: ' + c('lsblk') + ', ' + c('pericia imagem m-01') + ', ' + c('sudo mount -o ro,loop ~/casos/<ID>/M-01.dd /mnt/m01') + ', ' + c('grep -ril <termo> /mnt/m01') + '.',
    'Ao localizar a expressão, registre-a com ' + c('achei <expressão>') + ' e indique a origem.',
    'No Documento, use Pré-preencher, revise, escreva a conclusão separando fato de inferência, verifique a coerência e emita.',
    'Leia a Avaliação: cada ponto perdido traz o motivo.',
], 1):
    story.append(Paragraph(t, bullet, bulletText='%d.' % i))
story.append(Paragraph('7.4 COMANDOS DO SIMULADOR', h2))
quadro('Comandos do simulador (uso direto ou via pericia &lt;comando&gt; no Linux)', ['Grupo', 'Comandos'], [
    ['Situação', 'caso · itens · item &lt;id&gt; · tempo · notas · custodia · auditoria · progresso'],
    ['Físico', 'fotografar · lacre conferir|aplicar · deslacrar · abrir · componente remover · midia listar|remover · bloqueio · imagem · sanitizar · clone · hash · verificar-hash · energia desligar · memoria · volume copiar · rede isolar · inspecionar'],
    ['Móveis', 'isolar &lt;item&gt; faraday|aluminio · airplane · sim remover · alarmes · carregar · camera cobrir · solicitar-senhas · imei verificar · desbloquear &lt;item&gt; &lt;senha&gt; · extrair fisica|fs|logica · senha · tela &lt;item&gt; &lt;app&gt;|imei|foto'],
    ['Coleta', 'embalar · etiquetar'],
    ['Investigação', 'fs · recuperar · expandir · decifrar · ocr · dicionario · conexao · dns · whois · traceroute · navegar · preservar · arquivo-web · email · rede logs'],
    ['Relatório', 'anotar &lt;texto&gt; · achei &lt;expressão&gt; · laudo … · finalizar'],
], [0.18, 0.82])

# ------------------------------------------------------------------ 7 VERIFICAÇÃO
story.append(PageBreak())
story.append(Paragraph('8 VERIFICAÇÃO E TESTES', h1))
P('A verificação combina testes automatizados do motor, que jogam cada módulo do início ao fim pelo mesmo ' + c('exec') + ' usado '
  'pela interface, e roteiros da interface executados sob Xvfb com capturas de tela. O quadro a seguir lista as suítes; todas passam '
  'na versão %s (' % VERSAO + c('cargo test --workspace') + ').')
quadro('Suítes de testes automatizados', ['Suíte', 'Testes', 'O que verifica'], [
    ['Unitários do motor', '13', 'SHA-256 (vetores oficiais), PRNG, utilitários, auditoria (inclui colisão do formato antigo e reordenação).'],
    ['fluxos', '38', 'Reprodutibilidade, consistência das evidências, fluxos completos e falhas dos cinco POPs, eventos adversos, bateria, IMEI, isolamento de rede do código.'],
    ['linux', '6', 'Shell, curingas, montagens somente leitura, montagem do original, exame completo pelo terminal.'],
    ['celular', '9', 'Estados da tela, conteúdo exibido, regras de navegação no exame e na apreensão, *#06#, modo avião.'],
    ['nuvem', '5', 'Autorização judicial, requisição ao provedor, captura direta, bloqueio após tentativas, conexão institucional.'],
    ['extensoes', '6', 'Compatibilidade do gerador 1.0.0, cifragem por hardware com BIOS, arquivos danificados, assinatura × extensão, ARC.'],
    ['Interface (save, assets)', '10', 'Replay com aviso de divergência, salvamento antigo, versão desconhecida, gravação atômica, escape, leitor OBJ/MTL, validação de texturas.'],
    ['<b>Total</b>', '<b>87</b>', ''],
], [0.22, 0.1, 0.68])
P('Os roteiros de interface usam as variáveis ' + c('PERICIA_SCRIPT') + ', ' + c('PERICIA_SHOTS') + ' e ' + c('PERICIA_DATA') + ' '
  'para executar passos (novo caso, comandos, posição do jogador, aplicativos, celular) e salvar capturas; todas as figuras '
  'deste documento foram obtidas assim. Não há asserções automáticas sobre pixels.')

# ------------------------------------------------------------------ 8 DESEMPENHO
story.append(PageBreak())
story.append(Paragraph('9 DESEMPENHO', h1))
P('Medições feitas em CPU Intel Xeon de 2,80 GHz com 2 vCPUs e 8 GB de RAM, Linux x86_64, sem GPU (Xvfb e Mesa llvmpipe). '
  'Servem de linha de base, não de meta.')
quadro('Medições de desempenho', ['Medida', 'Resultado'], [
    ['Tamanho do executável', '≈ 4,5 MB (inclui fontes DejaVu embutidas)'],
    ['CPU por quadro, escritório em primeira pessoa, com sombras', 'v0.3.1: média 10,4 ms (p95 11,4); v0.4.0: média 10,9 ms (p95 12,6)'],
    ['CPU por quadro, estação com o editor aberto', '2,4 ms (p95 7,8 ms)'],
    ['Inicialização até o primeiro quadro', '0,38 s'],
    ['Memória máxima (RSS) com a cena 3D', '≈ 210–285 MB, majoritariamente do driver por software'],
    ['Geração de caso (média, 5 módulos)', '0,03 a 0,14 ms'],
    ['Comando do terminal (média, fluxo 5.03 D4)', '0,06 ms'],
    ['Geração do PDF do laudo (≈ 8,7 KB)', '0,54 ms'],
    ['Replay completo de um salvamento (14 comandos)', '0,80 ms'],
], [0.6, 0.4])
P('Técnicas aplicadas: arquivos de sistema sintetizados sob demanda (cerca de 120 mil por partição sem ocupar memória), '
  'persistência mínima por replay, um lote de malha por textura, texturas geradas uma vez na inicialização, modo 2D leve, '
  'economia de energia e opção de desligar as sombras.')

# ------------------------------------------------------------------ 9 LIMITAÇÕES
story.append(PageBreak())
story.append(Paragraph('10 LIMITAÇÕES E PENDÊNCIAS', h1))
bl([
    'Windows e macOS não foram compilados nem testados; o binário Linux pronto exige glibc 2.34 ou mais nova.',
    'RAID/JBOD não é modelado: exigiria mídias compostas no sistema de evidências, mudança estrutural que aguarda decisão.',
    'O PIN do chip SIM (3 tentativas, PUK 10) está no modelo e no manual, mas não tem comando.',
    'Modelos glTF/FBX/.blend ainda não são carregados; aguardam os arquivos e a avaliação. Texturas JPEG precisam de conversão para PNG.',
    'Mudanças de regras entre versões podem fazer um replay antigo divergir; o carregamento avisa, mas não recusa.',
    'O shell é um subconjunto do bash; a detecção de hipótese e a verificação de coerência são heurísticas por palavras-chave.',
    'A tela do celular é um modelo genérico, não reproduz um sistema real, e não tem teclado de desbloqueio.',
    'Hashes são calculados sobre a representação canônica do modelo, não sobre gigabytes reais.',
    'Os testes de interface são por roteiro e captura de tela, sem asserções sobre pixels.',
])

# ------------------------------------------------------------------ 10 CONSIDERAÇÕES
story.append(PageBreak())
story.append(Paragraph('11 CONSIDERAÇÕES FINAIS', h1))
P('O Perícia Sim transforma os POPs de informática forense em regras verificáveis: cada etapa tem consequência, cada ação deixa '
  'rastro auditável e cada caso pode ser reproduzido. A versão 0.3 acrescentou o ambiente em primeira pessoa com gráficos mais '
  'realistas, a estação sem senha e a tela funcional do aparelho celular; a versão 0.4.0 corrigiu falhas de integridade, completou '
  'nuvem, cifragem por hardware, BIOS, ARC e casos-limite de arquivos, separou fatos de hipóteses no laudo e preparou a entrada de '
  'modelos e texturas, mantendo o motor determinístico, isolado da rede e coberto por 87 testes automatizados. Os próximos passos '
  'recomendados são validar Windows e macOS em máquinas reais, decidir sobre o modelo de RAID e integrar, um a um, os assets quando chegarem.')

# ------------------------------------------------------------------ REFERÊNCIAS
story.append(PageBreak())
story.append(Paragraph('REFERÊNCIAS', h1u))
for r in [
    'ASSOCIAÇÃO BRASILEIRA DE NORMAS TÉCNICAS. <b>NBR 14724</b>: informação e documentação: trabalhos acadêmicos: apresentação. Rio de Janeiro: ABNT, 2011.',
    'ASSOCIAÇÃO BRASILEIRA DE NORMAS TÉCNICAS. <b>NBR 6023</b>: informação e documentação: referências: elaboração. Rio de Janeiro: ABNT, 2018.',
    'BRASIL. Ministério da Justiça e Segurança Pública. Secretaria Nacional de Segurança Pública. <b>Procedimento operacional padrão</b>: perícia criminal: informática forense. Brasília, DF: MJSP, 2024. v. 5.',
    'PETERS, V.; VAN DE WESTELAKEN, M. <b>Simulation games</b>: a concise introduction to the design process. Nijmegen: Samenspraak Advies, 2014.',
    'Q99 STUDIO. <b>How to build a simulation game</b>. [S. l.]: Q99 Studio, [2024?]. Disponível em: q99studio.com. Acesso em: 27 out. 2026.',
    'SUMMER ENGINE. <b>How to make a simulation game with AI</b>. [S. l.]: Summer Engine, [2025?]. Disponível em: summerengine.com. Acesso em: 27 out. 2026.',
]:
    story.append(Paragraph(r, ref))

# ------------------------------------------------------------------ APÊNDICE A
story.append(PageBreak())
story.append(Paragraph('APÊNDICE A – MATRIZ DE RASTREABILIDADE', h1u))
P('A matriz é gerada a partir de ' + c('crates/core/src/manual.rs') + ', fonte única, e liga cada requisito do POP ao comportamento '
  'do simulador, ao teste que o verifica e à limitação conhecida.')
rows = []
for line in open(os.path.join(ROOT, 'docs/matriz-rastreabilidade.md'), encoding='utf-8'):
    if line.startswith('| R'):
        cols = [x.strip() for x in line.strip().strip('|').split('|')]
        if len(cols) >= 6:
            fmt = lambda s: re.sub(r'`([^`]+)`', lambda m: '<font name="Mono" size="7">' + escape(m.group(1)) + '</font>', escape(s).replace('&amp;', '&'))
            def fmt2(s):
                parts = re.split(r'(`[^`]+`)', s)
                out = ''
                for p in parts:
                    if p.startswith('`'):
                        out += '<font name="Mono" size="7">' + escape(p[1:-1]) + '</font>'
                    else:
                        out += escape(p)
                return out
            rows.append([fmt2(x) for x in cols[:6]])
quadro('Matriz de rastreabilidade POP × simulador', ['ID', 'Fonte', 'Requisito', 'Comportamento', 'Teste', 'Limitação'], rows,
       [0.06, 0.12, 0.2, 0.28, 0.18, 0.16], small=True)

doc = Doc(OUT, pagesize=A4, title='Perícia Sim — documentação técnica %s' % VERSAO, author=AUTOR,
          subject='Simulador de treinamento em informática forense', leftMargin=3 * cm, rightMargin=2 * cm, topMargin=3 * cm, bottomMargin=2 * cm)
doc.multiBuild(story)
print('ok', doc.page)
