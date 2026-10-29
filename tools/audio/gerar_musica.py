"""Gera a música ambiente do Perícia Sim: jazz noir (composição e síntese originais — nada
é amostrado de gravações). O mesmo tema em três versões, tocadas pelo jogo em ordem aleatória:

  1. musica_noir.wav    — "Arquivo": 76 bpm, ré menor, sax tenor e vassourinhas (original)
  2. musica_noir_2.wav  — "Madrugada": balada a 62 bpm, dó menor, trompete com surdina harmon,
                          piano esparso, vassourinhas leves, sala mais reverberante
  3. musica_noir_3.wav  — "Chuva": 96 bpm, sol menor, vibrafone na melodia, baquetas no prato
                          de condução com notas fantasma na caixa, piano mais ativo, chuva ao fundo

Uso: python3 tools/audio/gerar_musica.py <pasta_assets> [1 2 3]   (sem números: as três)
Saída: <pasta_assets>/sounds/musica_noir*.wav — estéreo, 22,05 kHz, 16 bits, em loop sem emenda.

Formação: contrabaixo em walking, bateria com vassourinhas (caixa varrida, prato de condução
em swing, chimbal no 2 e no 4), piano elétrico (FM) em acordes sem fundamental e um sax
tenor sintetizado (dente de serra com formantes, sopro e vibrato) com a melodia.
Forma: introdução (4) · A (8) · A' (8) · B (8) · A (8) · final (4) = 40 compassos a 76 bpm.
"""

import os
import sys
import wave

import numpy as np
from scipy import signal

SR = 44100
BPM = 76.0
BEAT = 60.0 / BPM
SWING = 2.0 / 3.0  # colcheia "atrasada": a segunda colcheia cai em 2/3 do tempo
rng = np.random.default_rng(1949)

VERSOES = {
    1: dict(arquivo="musica_noir.wav", bpm=76.0, tp=0, solo="sax", bateria="vassourinha", piano="charleston", rt=1.9, chuva=False, seed=1949),
    2: dict(arquivo="musica_noir_2.wav", bpm=62.0, tp=-2, solo="trompete", bateria="suave", piano="esparso", rt=2.7, chuva=False, seed=1962, ganho_db=-2.0),
    3: dict(arquivo="musica_noir_3.wav", bpm=96.0, tp=5, solo="vibrafone", bateria="baquetas", piano="ativo", rt=1.6, chuva=True, seed=1996, ganho_db=0.4),
}
V = VERSOES[1]


def aplicar(v):
    """Ajusta andamento, swing e semente globais para a versão."""
    global BPM, BEAT, TOTAL, rng, V
    V = v
    BPM = v["bpm"]
    BEAT = 60.0 / BPM
    TOTAL = BARS * 4 * BEAT
    rng = np.random.default_rng(v["seed"])


def midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def t_of(beat):
    """Tempo (s) de uma posição em tempos, aplicando o swing às colcheias."""
    b = np.floor(beat)
    frac = beat - b
    sw = np.where(frac < 0.5, frac / 0.5 * SWING, SWING + (frac - 0.5) / 0.5 * (1 - SWING))
    return (b + sw) * BEAT


def band(x, lo=None, hi=None, order=2):
    if lo and hi:
        sos = signal.butter(order, [lo, hi], btype="band", fs=SR, output="sos")
    elif lo:
        sos = signal.butter(order, lo, btype="high", fs=SR, output="sos")
    else:
        sos = signal.butter(order, hi, btype="low", fs=SR, output="sos")
    return signal.sosfilt(sos, x)


def place(dst, src, t, g=1.0):
    i = int(t * SR)
    if i < 0 or i >= len(dst):
        return
    n = min(len(src), len(dst) - i)
    dst[i:i + n] += src[:n] * g


# ------------------------------------------------------------------ harmonia e forma

# acorde: (fundamental MIDI do baixo, notas do acorde para o walking, voicing do piano)
CH = {
    "Dm9": (38, [38, 41, 45, 48, 50, 52], [53, 57, 60, 64]),
    "Gm9": (43, [43, 46, 50, 53, 55, 57], [58, 62, 65, 69]),
    "Em7b5": (40, [40, 43, 46, 50, 52], [55, 58, 62, 64]),
    "A7b9": (45, [45, 49, 52, 55, 57, 58], [55, 58, 61, 65]),
    "Bbmaj7": (46, [46, 50, 53, 57, 58], [57, 60, 62, 65]),
    "Gm7": (43, [43, 46, 50, 53, 55], [53, 58, 62, 65]),
    "A7alt": (45, [45, 49, 51, 53, 55, 57], [55, 58, 61, 65]),
}
A1 = ["Dm9", "Dm9", "Gm9", "Gm9", "Em7b5", "A7b9", "Dm9", "A7b9"]
A2 = ["Dm9", "Dm9", "Gm9", "Gm9", "Em7b5", "A7b9", "Dm9", "Dm9"]
BB = ["Bbmaj7", "Bbmaj7", "Gm7", "Gm7", "Em7b5", "A7b9", "Dm9", "A7alt"]
INTRO = ["Dm9", "Bbmaj7", "Em7b5", "A7b9"]
FORMA = INTRO + A1 + A2 + BB + A1 + ["Dm9", "Bbmaj7", "Em7b5", "A7b9"]
BARS = len(FORMA)
TOTAL = BARS * 4 * BEAT

# melodia (tempo dentro do compasso de 8, duração em tempos, nota MIDI)
MEL_A = [
    (1.0, 0.67, 57), (1.67, 0.33, 60), (2.0, 1.6, 62),
    (4.0, 0.67, 65), (4.67, 0.33, 64), (5.0, 0.67, 62), (5.67, 1.8, 57),
    (8.67, 0.33, 58), (9.0, 0.67, 62), (9.67, 0.33, 65), (10.0, 1.5, 64), (11.67, 0.33, 62),
    (12.0, 0.67, 65), (12.67, 0.33, 67), (13.0, 2.4, 69),
    (16.67, 0.33, 55), (17.0, 0.67, 58), (17.67, 0.33, 62), (18.0, 1.0, 60), (19.0, 0.67, 58), (19.67, 0.33, 57),
    (20.0, 0.67, 58), (20.67, 0.33, 57), (21.0, 0.67, 55), (21.67, 0.33, 53), (22.0, 1.0, 52), (23.0, 0.67, 49),
    (24.0, 3.2, 50),
    (30.0, 0.67, 52), (30.67, 0.33, 53), (31.0, 0.67, 55), (31.67, 0.33, 57),
]
MEL_A2_FIM = [(28.0, 0.67, 53), (28.67, 0.33, 52), (29.0, 2.5, 50)]
MEL_B = [
    (0.67, 0.33, 65), (1.0, 0.67, 69), (1.67, 0.33, 72), (2.0, 2.0, 74),
    (4.67, 0.33, 72), (5.0, 0.67, 69), (5.67, 0.33, 67), (6.0, 1.5, 65),
    (8.0, 0.67, 67), (8.67, 0.33, 70), (9.0, 1.0, 74), (10.0, 0.67, 72), (10.67, 0.33, 70), (11.0, 1.5, 69),
    (13.0, 0.67, 65), (13.67, 0.33, 67), (14.0, 1.6, 62),
    (16.0, 0.67, 64), (16.67, 0.33, 67), (17.0, 0.67, 70), (17.67, 0.33, 69), (18.0, 1.6, 67),
    (20.0, 0.67, 69), (20.67, 0.33, 70), (21.0, 0.67, 73), (21.67, 0.33, 70), (22.0, 1.0, 69), (23.0, 1.0, 64),
    (24.0, 2.5, 62), (27.0, 0.67, 65), (27.67, 0.33, 64),
    (28.0, 0.67, 61), (28.67, 0.33, 62), (29.0, 0.67, 64), (29.67, 0.33, 65), (30.0, 1.0, 64), (31.0, 0.67, 61),
]


# ------------------------------------------------------------------ instrumentos

def contrabaixo(f, dur):
    n = int((dur + 0.4) * SR)
    t = np.arange(n) / SR
    bend = 1 + 0.012 * np.exp(-t / 0.03)  # ataque do dedo puxa a corda
    ph = 2 * np.pi * np.cumsum(f * bend) / SR
    x = np.zeros(n)
    for h, a, tau in [(1, 1.0, 0.9), (2, 0.55, 0.45), (3, 0.28, 0.25), (4, 0.12, 0.15), (5, 0.05, 0.1)]:
        x += a * np.sin(h * ph) * np.exp(-t / tau)
    thump = band(rng.standard_normal(n), 60, 400) * np.exp(-t / 0.012) * 0.6
    env = np.minimum(1, t / 0.006) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.06))
    return (x + thump) * env


def rhodes(f, dur, vel=1.0):
    n = int((dur + 1.2) * SR)
    t = np.arange(n) / SR
    idx = 1.1 * vel * np.exp(-t / 0.45)
    y = np.sin(2 * np.pi * f * t + idx * np.sin(2 * np.pi * f * t))
    y += 0.06 * vel * np.sin(2 * np.pi * f * 7.1 * t) * np.exp(-t / 0.05)
    trem = 1 + 0.12 * np.sin(2 * np.pi * 4.8 * t)
    env = np.minimum(1, t / 0.004) * np.exp(-t / 1.6) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.25))
    return y * env * trem * vel


def sax(notes_time, total_n):
    """Sax tenor: dente de serra limitado em banda, formantes, sopro, vibrato e portamento."""
    out = np.zeros(total_n)
    for (t0, dur, m, vel) in notes_time:
        f = midi_hz(m)
        n = int((dur + 0.15) * SR)
        t = np.arange(n) / SR
        scoop = -0.35 * np.exp(-t / 0.05)  # semitons: entra por baixo
        vib = 0.14 * np.sin(2 * np.pi * 5.3 * t) * np.clip((t - 0.22) / 0.25, 0, 1)
        fr = f * 2 ** ((scoop + vib) / 12)
        ph = 2 * np.pi * np.cumsum(fr) / SR
        nh = int(min(22, 5000 / f))
        bright = 0.55 + 0.45 * vel
        x = np.zeros(n)
        for h in range(1, nh + 1):
            x += (1.0 / h) * (bright ** (h * 0.35)) * np.sin(h * ph)
        att = np.minimum(1, t / 0.035)
        rel = np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.05))
        swell = 0.85 + 0.15 * np.minimum(1, t / 0.4)
        env = att * rel * swell * vel
        breath = band(rng.standard_normal(n), 1200, 4500) * 0.05 * env * (0.6 + 0.8 * np.exp(-t / 0.08))
        place(out, x * env + breath, t0)
    # formantes do corpo do instrumento e campana
    y = band(out, 380, 900) * 1.0 + band(out, 1200, 2000) * 0.55 + band(out, 2400, 3400) * 0.22 + band(out, hi=300) * 0.35
    return band(y, hi=5200)


def prato_conducao():
    n = int(1.8 * SR)
    t = np.arange(n) / SR
    x = band(rng.standard_normal(n), 3500, 12000) * np.exp(-t / 0.7) * 0.5
    for f in rng.uniform(2800, 9000, 12):
        x += 0.05 * np.sin(2 * np.pi * f * t + rng.uniform(0, 6)) * np.exp(-t / rng.uniform(0.4, 1.2))
    x += band(rng.standard_normal(n), 2000, 6000) * np.exp(-t / 0.01) * 0.6  # "ping" da baqueta
    return x


def vassourinha_tapa():
    n = int(0.25 * SR)
    t = np.arange(n) / SR
    return band(rng.standard_normal(n), 900, 7000) * np.exp(-t / 0.045)


def chimbal_pe():
    n = int(0.12 * SR)
    t = np.arange(n) / SR
    return band(rng.standard_normal(n), 5000, 12000) * np.exp(-t / 0.018) + band(rng.standard_normal(n), 400, 1200) * np.exp(-t / 0.01) * 0.3


def bumbo_pena():
    n = int(0.3 * SR)
    t = np.arange(n) / SR
    return np.sin(2 * np.pi * (52 + 30 * np.exp(-t / 0.02)) * t) * np.exp(-t / 0.12)


def reverb_ir(rt=1.9, seed=3):
    r = np.random.default_rng(seed)
    n = int(rt * SR)
    t = np.arange(n) / SR
    irs = []
    for c in range(2):
        ir = r.standard_normal(n) * np.exp(-t * 6.9 / rt)
        ir = band(ir, 200, 6500)
        ir[: int(0.018 * SR)] = 0
        irs.append(ir / np.sqrt(np.sum(ir ** 2)))
    return irs


def trompete(notes_time, total_n):
    """Trompete com surdina harmon: timbre fino e nasal, pouco vibrato, ataque com "língua"."""
    out = np.zeros(total_n)
    for (t0, dur, m, vel) in notes_time:
        f = midi_hz(m)
        n = int((dur + 0.12) * SR)
        t = np.arange(n) / SR
        bend = -0.25 * np.exp(-t / 0.03)
        vib = 0.08 * np.sin(2 * np.pi * 5.8 * t) * np.clip((t - 0.35) / 0.3, 0, 1)
        ph = 2 * np.pi * np.cumsum(f * 2 ** ((bend + vib) / 12)) / SR
        nh = int(min(28, 7000 / f))
        x = np.zeros(n)
        for h in range(1, nh + 1):
            x += (1.0 / h ** 0.8) * np.sin(h * ph)
        att = np.minimum(1, t / 0.02)
        rel = np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.04))
        env = att * rel * (0.8 + 0.2 * np.minimum(1, t / 0.5)) * vel
        lingua = band(rng.standard_normal(n), 2000, 6000) * np.exp(-t / 0.012) * 0.25
        place(out, x * env + lingua * vel, t0)
    # surdina harmon: corta graves e realça a região de 1,5 a 3 kHz
    y = band(out, 1300, 3300, order=3) * 1.0 + band(out, 700, 1100) * 0.35
    return band(y, hi=6500)


def vibrafone(notes_time, total_n):
    """Vibrafone: barras de metal (parciais 1 e 4) com motor do tremolo e baqueta macia."""
    out = np.zeros(total_n)
    for (t0, dur, m, vel) in notes_time:
        f = midi_hz(m)
        ring = max(dur, 0.6) + 1.4
        n = int(ring * SR)
        t = np.arange(n) / SR
        x = np.sin(2 * np.pi * f * t) * np.exp(-t / 1.6) + 0.22 * np.sin(2 * np.pi * f * 4.0 * t) * np.exp(-t / 0.25)
        x += 0.08 * np.sin(2 * np.pi * f * 10.0 * t) * np.exp(-t / 0.03)  # baqueta
        motor = 1 + 0.35 * np.sin(2 * np.pi * 5.5 * t)
        abafa = np.where(t < dur + 0.25, 1.0, np.exp(-(t - dur - 0.25) / 0.15))  # pedal solto depois da nota
        place(out, x * motor * abafa * np.minimum(1, t / 0.002) * vel, t0)
    return band(out, 150, 9000)


def caixa_fantasma():
    n = int(0.15 * SR)
    t = np.arange(n) / SR
    corpo = np.sin(2 * np.pi * 190 * t) * np.exp(-t / 0.03) * 0.5
    esteira = band(rng.standard_normal(n), 1800, 8000) * np.exp(-t / 0.05)
    return corpo + esteira


def chuva(n):
    """Chuva fina na janela: ruído rosa filtrado com gotas esparsas, em estéreo."""
    out = []
    for c in range(2):
        r = np.random.default_rng(77 + c)
        x = band(r.standard_normal(n), 400, 6000) * 0.010
        gotas = np.zeros(n)
        for i in r.integers(0, n, int(n / SR * 9)):
            gotas[i] = r.uniform(0.3, 1.0)
        x += band(gotas, 2500, 9000) * 0.018
        out.append(x)
    return out


# ------------------------------------------------------------------ arranjo

def walking(bar_idx):
    nome = FORMA[bar_idx]
    prox = FORMA[(bar_idx + 1) % BARS]
    raiz, tons, _ = CH[nome]
    alvo = CH[prox][0]
    r = np.random.default_rng(100 + bar_idx)
    linha = [raiz]
    for k in range(2):
        c = [x for x in tons if abs(x - linha[-1]) <= 7 and x != linha[-1]]
        linha.append(int(r.choice(c)) if c else raiz + 7)
    # aproximação cromática da fundamental do próximo compasso
    ap = alvo + (1 if linha[-1] > alvo else -1)
    if abs(ap - linha[-1]) > 9:
        ap = alvo - 1
    linha.append(ap)
    return linha


def render():
    n = int((TOTAL + 3.0) * SR)
    L = np.zeros(n)
    R = np.zeros(n)
    bass = np.zeros(n)
    piano = np.zeros(n)
    drumsL = np.zeros(n)
    drumsR = np.zeros(n)
    ride = prato_conducao()
    for b in range(BARS):
        base = b * 4
        # contrabaixo (no último compasso do loop, prepara a volta ao Dm9)
        tp = V["tp"]
        for k, m in enumerate(walking(b)):
            # transposições para cima descem uma oitava no baixo (fica na região do contrabaixo)
            mb = m + tp - (12 if tp > 2 else 0)
            place(bass, contrabaixo(midi_hz(mb), BEAT * 0.92), t_of(base + k), 0.9 + 0.1 * (k == 0))
        estilo = V["bateria"]
        # bateria: condução 1, 2, 2a, 3, 4, 4a (swing) e chimbal no 2 e 4
        if estilo != "suave":
            gr = 0.32 if estilo == "baquetas" else 0.22
            for pos, g in [(0, 0.55), (1, 0.7), (1.5, 0.4), (2, 0.55), (3, 0.7), (3.5, 0.4)]:
                place(drumsR, ride, t_of(base + pos), g * gr * (1 + rng.uniform(-0.1, 0.1)))
        for pos in (1, 3):
            place(drumsL, chimbal_pe(), t_of(base + pos), 0.16 if estilo == "baquetas" else 0.12)
            if estilo != "baquetas":
                place(drumsL, vassourinha_tapa(), t_of(base + pos), 0.12 if estilo == "suave" else 0.22)
        if estilo == "baquetas":
            # notas fantasma na caixa (baqueta) e bumbo marcando 1 e 3
            for pos in (0.5, 1.5, 2.5, 3.5):
                if rng.uniform() < 0.55:
                    place(drumsL, caixa_fantasma(), t_of(base + pos), rng.uniform(0.05, 0.09))
            if b % 4 == 3:
                place(drumsL, caixa_fantasma(), t_of(base + 3.5), 0.22)
            for pos in (0, 2):
                place(drumsL, bumbo_pena(), t_of(base + pos), 0.14)
        else:
            for pos in range(4):
                place(drumsL, bumbo_pena(), t_of(base + pos), 0.05)
            # varrida contínua da vassourinha (círculos, um por tempo)
            seg = int(4 * BEAT * SR)
            t = np.arange(seg) / SR
            sw = band(rng.standard_normal(seg), 1500, 8000) * (0.5 + 0.5 * np.abs(np.sin(np.pi * t / BEAT))) * (0.022 if estilo == "suave" else 0.035)
            place(drumsL, sw, t_of(base))
        # piano elétrico: Charleston e antecipações, sem fundamental
        _, _, voic = CH[FORMA[b]]
        ritmos = [(0, 1.2, 0.8), (1.5, 0.4, 0.6)] if b % 2 == 0 else [(0.5, 0.4, 0.55), (2.5, 1.2, 0.7)]
        if V["piano"] == "esparso":
            ritmos = [(0, 3.6, 0.7)] if b % 2 == 0 else [(0, 1.8, 0.6), (2.5, 1.2, 0.5)]
        elif V["piano"] == "ativo":
            ritmos = [(0, 0.8, 0.75), (1.5, 0.4, 0.6), (2.5, 0.4, 0.55), (3.5, 0.4, 0.6)] if b % 2 == 0 else [(0.5, 0.4, 0.6), (1.5, 0.8, 0.65), (3.0, 0.8, 0.6)]
        if b < 4 or b >= BARS - 4:
            ritmos = [(0, 3.5, 0.75)]
        for pos, d, vel in ritmos:
            for m in voic:
                place(piano, rhodes(midi_hz(m + V["tp"] - (12 if V["tp"] > 2 else 0)), d * BEAT, vel), t_of(base + pos) + rng.uniform(0, 0.012), 0.11)
    # melodia do sax por seção
    sec_start = {"A1": 4, "A2": 12, "B": 20, "A3": 28}
    notas = []
    for sec, ini in sec_start.items():
        mel = MEL_B if sec == "B" else (MEL_A[:-4] + MEL_A2_FIM if sec == "A2" else MEL_A)
        for (pos, d, m) in mel:
            vel = 0.75 + 0.25 * ((m - 50) / 25)
            notas.append((t_of(ini * 4 + pos), d * BEAT * 0.97, m, float(np.clip(vel, 0.6, 1.0))))
    # final: frase curta e nota longa no Dm9
    notas += [(t_of(36 * 4 + 1.0), 0.67 * BEAT, 57, 0.7), (t_of(36 * 4 + 1.67), 0.33 * BEAT, 60, 0.7), (t_of(36 * 4 + 2.0), 5.0 * BEAT, 62, 0.75)]
    # transposição e região do solista (trompete uma oitava acima do sax; vibrafone também)
    oit = {"sax": 0, "trompete": 12, "vibrafone": 12}[V["solo"]]
    notas = [(t0, d, m + V["tp"] + oit - (12 if V["tp"] > 2 else 0), vel) for (t0, d, m, vel) in notas]
    sx = {"sax": sax, "trompete": trompete, "vibrafone": vibrafone}[V["solo"]](notas, n)

    # equilíbrio por intensidade (RMS dos trechos ativos): o sax à frente, o contrabaixo
    # firme logo abaixo, piano e bateria como cama
    def ativo(x):
        w = SR // 10
        fr = np.array([np.sqrt(np.mean(x[i:i + w] ** 2)) for i in range(0, len(x) - w, w)])
        on = fr[fr > fr.max() * 0.1]
        return float(np.sqrt(np.mean(on ** 2))) if len(on) else 1.0

    def nivel(x, db):
        return x * (10 ** (db / 20) / ativo(x))

    sx = nivel(sx, 0.0)
    bass = nivel(bass, -6.0)
    piano = nivel(piano, -10.0)
    drumsR = nivel(drumsR, -16.0)
    drumsL = nivel(drumsL, -15.0)
    # mixagem com posicionamento estéreo
    L += bass * 0.7 + piano * 0.75 + drumsL * 1.0 + drumsR * 0.5 + sx * 0.9
    R += bass * 0.7 + piano * 1.0 + drumsL * 0.6 + drumsR * 1.0 + sx * 0.7
    if V["chuva"]:
        chv = chuva(n)
        L += chv[0]
        R += chv[1]
    irL, irR = reverb_ir(V["rt"])
    wetL = signal.fftconvolve(L, irL)[:n]
    wetR = signal.fftconvolve(R, irR)[:n]
    L = L + wetL * 0.28
    R = R + wetR * 0.28
    # dobra a cauda do fim sobre o início: loop contínuo
    m = int(TOTAL * SR)
    tail = n - m
    L[:tail] += L[m:]
    R[:tail] += R[m:]
    L, R = L[:m], R[:m]
    # cor de disco antigo: leve passa-baixas, chiado e estalos raros
    L = band(L, hi=9000)
    R = band(R, hi=9000)
    hiss = band(rng.standard_normal(m), 2000, 9000) * 0.0025
    crackle = np.zeros(m)
    for i in rng.integers(0, m, int(TOTAL * 3)):
        crackle[i] = rng.uniform(-1, 1) * 0.05
    crackle = band(crackle, 1500, 9000)
    L += hiss + crackle
    R += hiss + crackle * 0.8
    # nível: RMS ≈ -20 dBFS com teto de pico
    st = np.stack([L, R])
    rms = np.sqrt(np.mean(st ** 2))
    st *= min(10 ** (-20 / 20) / rms, 0.89 / np.abs(st).max())
    return st


def main():
    out = os.path.join(sys.argv[1] if len(sys.argv) > 1 else "assets", "sounds")
    os.makedirs(out, exist_ok=True)
    quais = [int(x) for x in sys.argv[2:]] or sorted(VERSOES)
    for k in quais:
        v = VERSOES[k]
        aplicar(v)
        st = render()
        # mesma intensidade percebida nas três versões (medida em LUFS; a do trompete soa mais alta)
        st = st * 10 ** (v.get("ganho_db", 0.0) / 20)
        st = signal.resample_poly(st, 1, 2, axis=1)  # 22,05 kHz basta para o clima de disco antigo
        data = (np.clip(st, -1, 1) * 32767).astype("<i2").T.reshape(-1)
        with wave.open(os.path.join(out, v["arquivo"]), "wb") as w:
            w.setnchannels(2)
            w.setsampwidth(2)
            w.setframerate(SR // 2)
            w.writeframes(data.tobytes())
        print("%s: %d compassos, %.1f s a %.0f bpm, pico %.1f dBFS" % (v["arquivo"], BARS, TOTAL, BPM, 20 * np.log10(np.abs(st).max())))


if __name__ == "__main__":
    main()
