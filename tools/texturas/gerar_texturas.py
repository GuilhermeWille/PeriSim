"""Gera as imagens de decoração e de vestígios do Perícia Sim (PIL, sem fotos de terceiros).

Uso: python3 tools/texturas/gerar_texturas.py <pasta_assets>
Saída: <pasta_assets>/textures/<nome>.png — dimensões em potência de 2 (exigência do jogo).
As imagens são desenhadas na proporção do quadro e reamostradas para a potência de 2; no
modelo, o UV 0..1 devolve a proporção certa.
"""

import math
import os
import random
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

FONTES = os.path.join(os.path.dirname(__file__), "..", "..", "crates", "app", "assets")


def fonte(tam, negrito=False):
    return ImageFont.truetype(os.path.join(FONTES, "DejaVuSans-Bold.ttf" if negrito else "DejaVuSans.ttf"), tam)


def centro(d, xy, txt, f, cor):
    w = d.textlength(txt, font=f)
    d.text((xy[0] - w / 2, xy[1]), txt, font=f, fill=cor)


def salvar(img, nome, tam, out):
    img.convert("RGBA" if img.mode == "RGBA" else "RGB").resize(tam, Image.LANCZOS).save(os.path.join(out, nome + ".png"))
    print("textura", nome, tam)


def cartaz_pop(out):
    W, H = 600, 850
    im = Image.new("RGB", (W, H), (244, 246, 248))
    d = ImageDraw.Draw(im)
    d.rectangle((0, 0, W, 120), fill=(22, 52, 96))
    centro(d, (W / 2, 22), "CADEIA DE CUSTÓDIA", fonte(40, True), (255, 255, 255))
    centro(d, (W / 2, 76), "etapas do rastreamento do vestígio", fonte(20), (200, 214, 232))
    etapas = ["Reconhecimento", "Isolamento", "Fixação", "Coleta", "Acondicionamento", "Transporte", "Recebimento", "Processamento", "Armazenamento", "Descarte"]
    y = 150
    for k, e in enumerate(etapas):
        x0 = 70 if k % 2 == 0 else 150
        d.rounded_rectangle((x0, y, x0 + 380, y + 50), 10, fill=(255, 255, 255), outline=(22, 52, 96), width=3)
        d.ellipse((x0 + 10, y + 9, x0 + 42, y + 41), fill=(214, 120, 30))
        centro(d, (x0 + 26, y + 13), str(k + 1), fonte(20, True), (255, 255, 255))
        d.text((x0 + 56, y + 12), e, font=fonte(24, True), fill=(22, 52, 96))
        if k < len(etapas) - 1:
            d.line((x0 + 190, y + 50, (150 if k % 2 == 0 else 70) + 190, y + 66), fill=(120, 130, 145), width=3)
        y += 66
    centro(d, (W / 2, H - 40), "Registre cada movimentação · documento fictício de treinamento", fonte(15), (90, 98, 110))
    salvar(im, "cartaz_pop", (512, 1024), out)


def cartaz_epi(out):
    W, H = 500, 700
    im = Image.new("RGB", (W, H), (255, 214, 40))
    d = ImageDraw.Draw(im)
    d.rectangle((14, 14, W - 14, H - 14), outline=(20, 20, 20), width=8)
    centro(d, (W / 2, 40), "ATENÇÃO", fonte(54, True), (20, 20, 20))
    centro(d, (W / 2, 108), "EPI OBRIGATÓRIO", fonte(34, True), (20, 20, 20))
    # ícones simples: luva, pulseira, jaleco
    def circ(cx, cy):
        d.ellipse((cx - 60, cy - 60, cx + 60, cy + 60), fill=(22, 90, 170))
    circ(125, 250)
    d.rounded_rectangle((100, 215, 150, 290), 12, fill=(255, 255, 255))
    for k in range(4):
        d.rounded_rectangle((97 + k * 14, 190, 108 + k * 14, 232), 5, fill=(255, 255, 255))
    d.rounded_rectangle((143, 238, 172, 255), 6, fill=(255, 255, 255))
    circ(375, 250)
    d.ellipse((335, 215, 415, 285), outline=(255, 255, 255), width=10)
    d.line((415, 250, 440, 300), fill=(255, 255, 255), width=5)
    centro(d, (125, 320), "Luvas", fonte(24, True), (20, 20, 20))
    centro(d, (375, 320), "Pulseira ESD", fonte(24, True), (20, 20, 20))
    linhas = ["Calce luvas antes de tocar", "no material examinado.", "", "Troque as luvas depois de", "tocar vestígio biológico.", "", "Aterre a pulseira antes de", "abrir equipamentos."]
    y = 380
    for l in linhas:
        centro(d, (W / 2, y), l, fonte(24), (20, 20, 20))
        y += 32
    salvar(im, "cartaz_epi", (512, 1024), out)


def mao_livre(d, xy, txt, f, cor, rnd, ang=0.0):
    # texto "à mão" com pequena ondulação por letra
    x, y = xy
    for ch in txt:
        d.text((x, y + rnd.uniform(-1.5, 1.5)), ch, font=f, fill=cor)
        x += d.textlength(ch, font=f) * rnd.uniform(0.95, 1.05)


def quadro_branco(out):
    W, H = 1400, 850
    rnd = random.Random(7)
    im = Image.new("RGB", (W, H), (246, 247, 248))
    d = ImageDraw.Draw(im)
    for k in range(40):  # marcas de apagador
        x, y = rnd.uniform(0, W), rnd.uniform(0, H)
        d.ellipse((x, y, x + rnd.uniform(60, 200), y + rnd.uniform(10, 30)), fill=(236, 238, 240))
    azul, verm, verde = (30, 60, 150), (190, 30, 30), (20, 120, 60)
    mao_livre(d, (60, 40), "FILA DE EXAMES — semana", fonte(52, True), azul, rnd)
    fila = ["C501-D2 · gabinete lacrado · aquisição OK", "C502-D1 · celular · isolar + bateria!", "C503-D3 · local · memória antes de desligar", "C504-D2 · e-mail · conferir cabeçalho"]
    y = 140
    for k, l in enumerate(fila):
        d.rectangle((60, y + 12, 84, y + 36), outline=azul, width=4)
        if k == 0:
            d.line((62, y + 22, 72, y + 34, 90, y + 4), fill=verde, width=6)
        mao_livre(d, (104, y), l, fonte(38), azul, rnd)
        y += 70
    mao_livre(d, (60, y + 30), "NÃO ESQUECER:", fonte(44, True), verm, rnd)
    for l in ["verificar o hash da imagem", "luvas novas a cada vestígio", "lacre novo + etiqueta no fim"]:
        y += 62
        mao_livre(d, (100, y + 30), "→ " + l, fonte(38), verm, rnd)
    # pequeno diagrama
    d.rectangle((960, 460, 1120, 540), outline=azul, width=5)
    mao_livre(d, (978, 482), "mídia", fonte(34), azul, rnd)
    d.rectangle((1180, 460, 1340, 540), outline=azul, width=5)
    mao_livre(d, (1188, 482), "bloq.", fonte(34), azul, rnd)
    d.line((1120, 500, 1180, 500), fill=azul, width=5)
    d.rectangle((1070, 620, 1250, 700), outline=azul, width=5)
    mao_livre(d, (1088, 640), "estação", fonte(34), azul, rnd)
    d.line((1260, 540, 1200, 620), fill=azul, width=5)
    d.rectangle((W - 260, H - 30, W - 60, H - 14), fill=(40, 40, 40))  # apoio dos marcadores
    salvar(im, "quadro_branco", (1024, 512), out)


def cortica(out):
    W, H = 1200, 800
    rnd = random.Random(11)
    im = Image.new("RGB", (W, H), (176, 128, 80))
    px = im.load()
    for y in range(H):
        for x in range(0, W):
            n = rnd.random()
            if n < 0.25:
                c = px[x, y]
                k = rnd.randint(-38, 22)
                px[x, y] = (max(0, c[0] + k), max(0, c[1] + k), max(0, c[2] + k))
    im = im.filter(ImageFilter.GaussianBlur(0.7))
    d = ImageDraw.Draw(im)
    pins = []
    def foto(x, y, w, h, ang, tom):
        f = Image.new("RGBA", (w + 24, h + 60), (245, 243, 236, 255))
        fd = ImageDraw.Draw(f)
        fd.rectangle((12, 12, w + 12, h + 12), fill=tom)
        # silhueta genérica (pessoa/edifício) para não depender de imagem real
        if rnd.random() < 0.6:
            cx = 12 + w // 2
            fd.ellipse((cx - 22, 30, cx + 22, 74), fill=(40, 40, 46))
            fd.rounded_rectangle((cx - 42, 78, cx + 42, h + 12), 18, fill=(40, 40, 46))
        else:
            for k in range(4):
                bx = 20 + k * (w // 4)
                fd.rectangle((bx, 12 + h // 2 - k * 12, bx + w // 5, h + 12), fill=(50, 54, 60))
        fd.text((16, h + 22), rnd.choice(["?", "22h40", "Rua 7", "visto 2x", "carro escuro"]), font=fonte(20), fill=(30, 30, 30))
        f = f.rotate(ang, expand=True, resample=Image.BICUBIC)
        im.paste(f, (x, y), f)
        pins.append((x + f.width // 2, y + 14))
    foto(60, 60, 200, 150, 4, (120, 128, 140))
    foto(360, 40, 180, 140, -6, (140, 120, 110))
    foto(880, 70, 210, 150, 3, (110, 120, 128))
    foto(120, 420, 190, 140, -3, (128, 132, 120))
    foto(820, 430, 200, 150, 5, (118, 112, 128))
    # fichas e bilhetes
    for (x, y, t) in [(560, 300, "LINHA DO\nTEMPO"), (620, 520, "pendrive\nno chaveiro?"), (380, 560, "e-mail 03:12\nIP?")]:
        d.rectangle((x, y, x + 180, y + 110), fill=(250, 240, 150))
        d.multiline_text((x + 14, y + 18), t, font=fonte(24, True), fill=(60, 40, 20), spacing=6)
        pins.append((x + 90, y + 8))
    # barbante vermelho ligando os alfinetes
    ordem = [0, 5, 1, 6, 3, 7, 4, 2]
    for a, b in zip(ordem, ordem[1:]):
        d.line((pins[a], pins[b]), fill=(170, 20, 24), width=4)
    for p in pins:
        d.ellipse((p[0] - 9, p[1] - 9, p[0] + 9, p[1] + 9), fill=(200, 30, 30), outline=(90, 10, 10), width=2)
    salvar(im, "quadro_cortica", (1024, 512), out)


def pintura(out):
    W, H = 900, 600
    im = Image.new("RGB", (W, H))
    d = ImageDraw.Draw(im)
    for y in range(H):  # céu de fim de tarde
        t = y / H
        d.line((0, y, W, y), fill=(int(70 + 160 * t), int(80 + 90 * t), int(140 - 40 * t)))
    d.ellipse((600, 300, 720, 420), fill=(250, 200, 120))
    rnd = random.Random(5)
    for k, (base, cor) in enumerate([(420, (70, 70, 96)), (470, (52, 60, 70)), (520, (34, 44, 46))]):
        pts = [(0, H)]
        x = 0
        while x <= W:
            pts.append((x, base - 60 * math.sin(x / (120 + 40 * k) + k) - rnd.uniform(0, 20)))
            x += 30
        pts.append((W, H))
        d.polygon(pts, fill=cor)
    im = im.filter(ImageFilter.GaussianBlur(1.2))
    salvar(im, "pintura", (1024, 512), out)


def calendario(out):
    W, H = 350, 500
    im = Image.new("RGB", (W, H), (252, 252, 250))
    d = ImageDraw.Draw(im)
    d.rectangle((0, 0, W, 190), fill=(60, 90, 70))
    for k in range(6):
        d.polygon([(k * 70, 190), (k * 70 + 35, 120 + (k % 3) * 15), (k * 70 + 70, 190)], fill=(80 + k * 8, 120, 90))
    centro(d, (W / 2, 205), "OUTUBRO 2026", fonte(28, True), (40, 40, 40))
    dias = "D S T Q Q S S".split()
    for k, dd in enumerate(dias):
        centro(d, (28 + k * 49, 250), dd, fonte(16, True), (150, 40, 40) if k == 0 else (60, 60, 60))
    dia = 1
    for sem in range(5):
        for k in range(7):
            if sem == 0 and k < 4:
                continue
            if dia > 31:
                break
            centro(d, (28 + k * 49, 280 + sem * 40), str(dia), fonte(18), (150, 40, 40) if k == 0 else (40, 40, 40))
            if dia == 27:
                d.ellipse((8 + k * 49, 274 + sem * 40, 48 + k * 49, 308 + sem * 40), outline=(200, 30, 30), width=3)
            dia += 1
    salvar(im, "calendario", (512, 1024), out)


def placa_lab(out):
    W, H = 1300, 220
    im = Image.new("RGB", (W, H), (22, 52, 96))
    d = ImageDraw.Draw(im)
    d.rectangle((10, 10, W - 10, H - 10), outline=(200, 214, 232), width=4)
    centro(d, (W / 2, 50), "LABORATÓRIO DE INFORMÁTICA FORENSE", fonte(54, True), (255, 255, 255))
    centro(d, (W / 2, 140), "acesso restrito · material sob custódia", fonte(34), (200, 214, 232))
    salvar(im, "placa_lab", (1024, 256), out)


def relogio(out):
    S = 512
    im = Image.new("RGB", (S, S), (250, 250, 248))
    d = ImageDraw.Draw(im)
    c = S / 2
    d.ellipse((6, 6, S - 6, S - 6), outline=(30, 30, 30), width=10)
    for k in range(60):
        a = k / 60 * math.tau
        r0 = c - 30 if k % 5 == 0 else c - 22
        d.line((c + math.sin(a) * r0, c - math.cos(a) * r0, c + math.sin(a) * (c - 14), c - math.cos(a) * (c - 14)), fill=(30, 30, 30), width=8 if k % 5 == 0 else 3)
    for h in range(1, 13):
        a = h / 12 * math.tau
        centro(d, (c + math.sin(a) * (c - 70), c - math.cos(a) * (c - 70) - 22), str(h), fonte(40, True), (30, 30, 30))
    for (frac, comp, larg, cor) in [((10 + 10 / 60) / 12, 120, 14, (30, 30, 30)), (10 / 60, 190, 9, (30, 30, 30)), (38 / 60, 200, 3, (190, 30, 30))]:
        a = frac * math.tau
        d.line((c, c, c + math.sin(a) * comp, c - math.cos(a) * comp), fill=cor, width=larg)
    d.ellipse((c - 12, c - 12, c + 12, c + 12), fill=(30, 30, 30))
    salvar(im, "relogio", (512, 512), out)


def documento(out):
    W, H = 760, 1000
    im = Image.new("RGB", (W, H), (252, 252, 249))
    d = ImageDraw.Draw(im)
    d.text((60, 50), "POLÍCIA CIVIL — DELEGACIA DE REPRESSÃO", font=fonte(24, True), fill=(30, 30, 30))
    d.text((60, 84), "A CRIMES CIBERNÉTICOS (fictícia)", font=fonte(24, True), fill=(30, 30, 30))
    d.text((60, 150), "OFÍCIO Nº ____/2026", font=fonte(30, True), fill=(30, 30, 30))
    d.text((60, 210), "Encaminha material para exame pericial.", font=fonte(22), fill=(40, 40, 40))
    y = 270
    rnd = random.Random(3)
    for k in range(16):
        w = rnd.randint(420, 640)
        d.line((60, y, 60 + w, y), fill=(120, 120, 125), width=5)
        y += 34
    d.rectangle((60, y + 20, W - 60, y + 160), outline=(60, 60, 60), width=3)
    d.text((80, y + 36), "Material: 1 (um) equipamento — lacre nº ______", font=fonte(22), fill=(40, 40, 40))
    d.text((80, y + 80), "Quesitos em anexo.", font=fonte(22), fill=(40, 40, 40))
    d.line((W - 330, H - 120, W - 80, H - 120), fill=(40, 40, 40), width=3)
    d.text((W - 300, H - 108), "Autoridade requisitante", font=fonte(18), fill=(60, 60, 60))
    salvar(im, "documento", (512, 1024), out)


def mancha(out):
    """Mancha seca de aspecto de sangue, com gotas-satélite (RGBA, fundo transparente).
    Tom escuro e amarronzado de sangue seco — realista, sem exagero."""
    S = 512
    rnd = random.Random(21)
    a = Image.new("L", (S, S), 0)
    d = ImageDraw.Draw(a)
    c = S / 2
    pts = []
    for k in range(48):
        ang = k / 48 * math.tau
        r = 110 + 30 * math.sin(3 * ang + 1) + rnd.uniform(-12, 12)
        pts.append((c + math.cos(ang) * r, c + math.sin(ang) * r * 0.8))
    d.polygon(pts, fill=255)
    for k in range(26):  # gotas-satélite
        ang = rnd.uniform(0, math.tau)
        dist = rnd.uniform(140, 235)
        r = rnd.uniform(4, 16) * (1.4 - dist / 260)
        x, y = c + math.cos(ang) * dist, c + math.sin(ang) * dist * 0.85
        d.ellipse((x - r, y - r * 0.8, x + r, y + r * 0.8), fill=255)
    a = a.filter(ImageFilter.GaussianBlur(2.2))
    rgb = Image.new("RGB", (S, S))
    px = rgb.load()
    ap = a.load()
    for y in range(S):
        for x in range(S):
            dd = math.hypot(x - c, y - c) / 160
            k = rnd.uniform(-0.06, 0.06)
            # borda mais escura (anel de secagem), centro um pouco mais claro
            edge = max(0.0, 1.0 - abs(ap[x, y] / 255 - 0.6) * 2.5)
            r_ = 0.30 + 0.06 * min(dd, 1) + k - edge * 0.08
            px[x, y] = (int(max(0, r_) * 255), int(max(0, 0.07 + k * 0.3) * 255), int(max(0, 0.06 + k * 0.3) * 255))
    a = a.point(lambda v: int(v * 0.92))
    rgb.putalpha(a)
    rgb.save(os.path.join(out, "mancha.png"))
    print("textura mancha", (S, S))


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else "assets"
    out = os.path.join(base, "textures")
    os.makedirs(out, exist_ok=True)
    for fn in (cartaz_pop, cartaz_epi, quadro_branco, cortica, pintura, calendario, placa_lab, relogio, documento, mancha):
        fn(out)


if __name__ == "__main__":
    main()
