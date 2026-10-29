"""Gera o banco de sons do Perícia Sim por síntese (nenhuma gravação de terceiros).

Uso: python3 tools/audio/gerar_sons.py <pasta_assets>
Saída: <pasta_assets>/sounds/<nome>.wav — PCM 16 bits, 44,1 kHz (mono; ambientes em estéreo).

Técnicas: ruído filtrado com envelopes (atrito, papel, fita, tecido), síntese modal
(senoides amortecidas: batidas em plástico, metal e madeira), varreduras (motores,
ventoinhas), e uma resposta de sala curta e sintética para dar corpo aos efeitos.
Cada som usa semente fixa: rodar de novo produz arquivos idênticos.
"""

import os
import sys
import wave

import numpy as np
from scipy import signal

SR = 44100


# ---------------------------------------------------------------- utilidades

class Sig(np.ndarray):
    """Sinal cuja soma com outro de comprimento diferente completa o menor com silêncio."""

    def _pad(self, o):
        a = np.asarray(self)
        b = np.asarray(o)
        if b.ndim == 0 or a.shape == b.shape:
            return a, b
        n = max(len(a), len(b))
        return np.pad(a, (0, n - len(a))), np.pad(b, (0, n - len(b)))

    def __add__(self, o):
        a, b = self._pad(o)
        return (a + b).view(Sig)

    __radd__ = __add__


def rng(seed):
    return np.random.default_rng(seed)


def t_axis(dur):
    return np.arange(int(dur * SR)) / SR


def silence(dur):
    return np.zeros(int(dur * SR))


def band(x, lo=None, hi=None, order=4):
    if lo and hi:
        sos = signal.butter(order, [lo, hi], btype="band", fs=SR, output="sos")
    elif lo:
        sos = signal.butter(order, lo, btype="high", fs=SR, output="sos")
    else:
        sos = signal.butter(order, hi, btype="low", fs=SR, output="sos")
    return signal.sosfilt(sos, x)


def pink(n, r):
    """Ruído rosa (1/f) por filtragem espectral."""
    w = r.standard_normal(n)
    f = np.fft.rfft(w)
    k = np.arange(len(f))
    k[0] = 1
    f /= np.sqrt(k)
    x = np.fft.irfft(f, n)
    return x / (np.abs(x).max() + 1e-9)


def brown(n, r):
    x = np.cumsum(r.standard_normal(n))
    x = band(x, lo=20, order=2)
    return x / (np.abs(x).max() + 1e-9)


def env_exp(n, tau, attack=0.002):
    t = np.arange(n) / SR
    e = np.exp(-t / tau)
    a = int(attack * SR)
    if a > 0:
        e[:a] *= np.linspace(0, 1, a)
    return e


def env_ar(n, attack, release):
    e = np.ones(n)
    a, r_ = int(attack * SR), int(release * SR)
    if a:
        e[:a] = np.linspace(0, 1, a) ** 2
    if r_:
        e[-r_:] *= np.linspace(1, 0, r_) ** 2
    return e


def modal(freqs, taus, amps, dur, r=None, detune=0.0):
    """Soma de modos amortecidos (batidas em objetos sólidos)."""
    t = t_axis(dur)
    x = np.zeros_like(t)
    for f, tau, a in zip(freqs, taus, amps):
        if r is not None and detune:
            f *= 1 + r.uniform(-detune, detune)
        ph = r.uniform(0, 2 * np.pi) if r is not None else 0
        x += a * np.sin(2 * np.pi * f * t + ph) * np.exp(-t / tau)
    a = int(0.0008 * SR)
    x[:a] *= np.linspace(0, 1, a)
    return x.view(Sig)


def click(r, dur=0.012, lo=1500, hi=9000, tau=0.002):
    n = int(dur * SR)
    return (band(r.standard_normal(n), lo, hi) * env_exp(n, tau, 0.0002)).view(Sig)


def place(dst, src, at, gain=1.0):
    i = int(at * SR)
    if i >= len(dst):
        return dst
    m = min(len(src), len(dst) - i)
    dst[i:i + m] += src[:m] * gain
    return dst


def tone(f, dur, r=None, harm=(1.0,), att=0.005, rel=0.05):
    t = t_axis(dur)
    x = sum(a * np.sin(2 * np.pi * f * (k + 1) * t) for k, a in enumerate(harm))
    return x * env_ar(len(t), att, min(rel, dur / 2))


def sweep(f0, f1, dur, curve="exp"):
    t = t_axis(dur)
    if curve == "exp":
        f = f0 * (f1 / f0) ** (t / dur)
    else:
        f = f0 + (f1 - f0) * t / dur
    ph = 2 * np.pi * np.cumsum(f) / SR
    return np.sin(ph)


def room(x, r, rt=0.22, wet=0.12, bright=5000):
    """Sala pequena sintética (ruído com decaimento exponencial)."""
    n = int(rt * SR * 1.6)
    ir = r.standard_normal(n) * np.exp(-np.arange(n) / (rt * SR / 6.9))
    ir = band(ir, 120, bright, order=2)
    ir[: int(0.006 * SR)] = 0  # pré-atraso
    ir /= np.sqrt(np.sum(ir ** 2)) + 1e-9
    y = signal.fftconvolve(x, ir)[: len(x) + n]
    out = np.zeros(len(y))
    out[: len(x)] += x
    return out + y * wet


def grains(dur, r, rate, lo, hi, glen=(0.004, 0.02), amp=(0.3, 1.0), shape=None):
    """Nuvem de grãos de ruído (papel, plástico, alumínio, tecido)."""
    x = silence(dur)
    t = 0.0
    while t < dur:
        t += r.exponential(1.0 / rate)
        g = int(r.uniform(*glen) * SR)
        if g < 8:
            continue
        lo_ = lo * r.uniform(0.8, 1.25)
        hi_ = min(hi * r.uniform(0.8, 1.2), SR / 2 - 500)
        gr = band(r.standard_normal(g), lo_, hi_, order=2) * env_exp(g, g / SR / 3, 0.0003)
        a = r.uniform(*amp)
        if shape is not None:
            a *= shape(min(t / dur, 1.0))
        place(x, gr, t, a)
    return x


def trim_tail(x, thresh_db=-58):
    """Corta o silêncio final (cauda da sala abaixo do limiar), mantendo 20 ms."""
    a = np.abs(x)
    m = a.max()
    if m == 0:
        return x
    idx = np.nonzero(a > m * 10 ** (thresh_db / 20))[0]
    end = min(len(x), idx[-1] + int(0.02 * SR)) if len(idx) else len(x)
    return x[:end]


def active_rms(x):
    """RMS só dos trechos com som (janelas de 50 ms acima de -35 dB da mais forte)."""
    w = int(0.05 * SR)
    n = max(1, len(x) // w)
    fr = np.array([np.sqrt(np.mean(x[i * w:(i + 1) * w] ** 2)) for i in range(n)])
    on = fr[fr > fr.max() * 10 ** (-35 / 20)]
    return float(np.sqrt(np.mean(on ** 2))) if len(on) else 0.0


def finish(x, peak=0.89, fade=0.004, rms_db=-21.0):
    """Normaliza pela intensidade percebida (RMS ativo) com teto de pico: um ruído
    de papel e um clique curto ficam com volume subjetivo parecido."""
    x = trim_tail(np.asarray(x, dtype=np.float64))
    x = x - np.mean(x) if x.ndim == 1 else x
    m = np.abs(x).max()
    if m > 0:
        g_rms = 10 ** (rms_db / 20) / max(active_rms(x), 1e-9)
        x = x * min(peak / m, g_rms)
    f = int(fade * SR)
    if f and x.shape[-1] > 2 * f:
        ramp = np.linspace(0, 1, f)
        if x.ndim == 1:
            x[:f] *= ramp
            x[-f:] *= ramp[::-1]
        else:
            x[:, :f] *= ramp
            x[:, -f:] *= ramp[::-1]
    return x


def write(path, x):
    x = np.clip(x, -1, 1)
    ch = 1 if x.ndim == 1 else x.shape[0]
    data = (x * 32767).astype("<i2")
    if ch == 2:
        data = np.stack([data[0], data[1]], axis=1).reshape(-1)
    with wave.open(path, "wb") as w:
        w.setnchannels(ch)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(data.tobytes())


# ---------------------------------------------------------------- sons

SONS = {}


def som(nome, peak=0.85):
    def reg(fn):
        SONS[nome] = (fn, peak)
        return fn
    return reg


# --- tablet

def tecido(r, dur, intens=1.0):
    shape = lambda u: np.sin(np.pi * u) ** 0.7
    x = grains(dur, r, 260, 700, 6000, glen=(0.006, 0.03), amp=(0.1, 0.6), shape=shape)
    x += band(r.standard_normal(len(x)), 300, 2500, 2) * 0.05 * np.sin(np.pi * np.linspace(0, 1, len(x)))
    return x * intens


def baque_borracha(r, dur=0.25):
    return modal([140, 260, 470], [0.05, 0.03, 0.015], [1.0, 0.5, 0.25], dur, r, 0.03) + band(click(r, 0.02, 200, 1500, 0.004), hi=1500) * 0.4


def chime(fs, dur_each=0.16, gap=0.07, level=1.0):
    x = silence(len(fs) * (dur_each + gap) + 0.3)
    for k, f in enumerate(fs):
        place(x, tone(f, dur_each + 0.25, harm=(1.0, 0.15), att=0.004, rel=0.2) * env_exp(int((dur_each + 0.25) * SR), 0.12), k * (dur_each + gap), level)
    return x


@som("tablet_sacar")
def _(r):
    x = silence(1.05)
    place(x, tecido(r, 0.5), 0.0, 0.8)
    place(x, baque_borracha(r), 0.46, 0.9)
    place(x, chime([988, 1319], 0.09, 0.03), 0.62, 0.22)
    return room(x, r, 0.18, 0.1)


@som("tablet_guardar")
def _(r):
    x = silence(0.95)
    place(x, chime([1319, 988], 0.08, 0.03), 0.0, 0.18)
    place(x, tecido(r, 0.45, 0.9), 0.2, 0.8)
    place(x, baque_borracha(r) * 0.7, 0.62, 0.7)
    return room(x, r, 0.18, 0.1)


@som("tablet_toque", peak=0.5)
def _(r):
    x = silence(0.12)
    place(x, click(r, 0.01, 2500, 9000, 0.0015), 0.0, 1.0)
    place(x, modal([320, 900], [0.01, 0.006], [0.4, 0.2], 0.05, r), 0.0, 1.0)
    return x


@som("nota_gravada", peak=0.45)
def _(r):
    return chime([784, 1175], 0.08, 0.02)


# --- interface

@som("ui_clique", peak=0.35)
def _(r):
    x = silence(0.06)
    place(x, click(r, 0.008, 2000, 7000, 0.0012), 0.0)
    place(x, modal([1800], [0.006], [0.3], 0.03, r), 0.0)
    return x


@som("ui_erro", peak=0.45)
def _(r):
    x = silence(0.42)
    for k, f in enumerate([233, 196]):
        place(x, tone(f, 0.15, harm=(1.0, 0.5, 0.3, 0.15), att=0.004, rel=0.06), k * 0.16)
    return band(x, hi=3000, order=2)


@som("aviso", peak=0.4)
def _(r):
    return chime([880], 0.12, 0.0)


# --- passos (dois pisos, quatro variações cada)

def passo(r, piso):
    x = silence(0.32)
    if piso == "vinil":
        heel = band(r.standard_normal(int(0.05 * SR)), 50, 400, 2) * env_exp(int(0.05 * SR), 0.012)
        body = modal([95, 180], [0.03, 0.02], [1.0, 0.4], 0.12, r, 0.08)
        scuff = band(r.standard_normal(int(0.07 * SR)), 1500, 6000, 2) * env_ar(int(0.07 * SR), 0.02, 0.04)
        place(x, heel, 0.0, 1.0)
        place(x, body, 0.0, 0.5)
        place(x, scuff, 0.05 + r.uniform(0, 0.02), 0.12)
        place(x, click(r, 0.01, 800, 4000, 0.002), 0.085 + r.uniform(0, 0.015), 0.25)  # ponta do pé
    else:  # madeira
        heel = band(r.standard_normal(int(0.04 * SR)), 60, 900, 2) * env_exp(int(0.04 * SR), 0.01)
        body = modal([140, 245, 410, 690], [0.06, 0.04, 0.025, 0.012], [1.0, 0.6, 0.35, 0.15], 0.2, r, 0.06)
        place(x, heel, 0.0, 0.9)
        place(x, body, 0.0, 0.55)
        place(x, click(r, 0.012, 1000, 5000, 0.002), 0.08 + r.uniform(0, 0.02), 0.3)
    return room(band(x, hi=7000, order=2), r, 0.25, 0.14)


for _k in range(4):
    SONS[f"passo_vinil_{_k + 1}"] = ((lambda k: lambda r: passo(r, "vinil"))(_k), 0.7)
    SONS[f"passo_madeira_{_k + 1}"] = ((lambda k: lambda r: passo(r, "madeira"))(_k), 0.7)


@som("cadeira")
def _(r):
    x = silence(1.0)
    # rangido: pulsos de atrito com período variável, filtrados como ressonância do metal
    t = 0.12
    period = 0.022
    while t < 0.55:
        place(x, band(click(r, 0.006, 600, 3500, 0.0015), 900, 2600, 2), t, r.uniform(0.3, 0.6))
        period *= r.uniform(0.93, 1.06)
        t += period
    place(x, tecido(r, 0.5, 0.7), 0.05, 0.6)
    place(x, modal([70, 130, 210], [0.12, 0.08, 0.05], [1.0, 0.5, 0.3], 0.4, r), 0.5, 0.9)  # assento
    return room(x, r, 0.25, 0.15)


# --- interação com objetos (pegar / tocar)

@som("interagir_gabinete")
def _(r):
    x = silence(0.6)
    place(x, modal([210, 470, 880, 1330, 2100], [0.18, 0.14, 0.09, 0.06, 0.03], [1.0, 0.7, 0.45, 0.3, 0.15], 0.6, r, 0.02), 0.0)
    place(x, click(r, 0.015, 800, 6000, 0.003), 0.0, 0.8)
    place(x, modal([95], [0.08], [0.8], 0.3, r), 0.0)  # corpo
    return room(x, r, 0.22, 0.12)


@som("interagir_notebook")
def _(r):
    x = silence(0.4)
    place(x, modal([820, 1650, 2700, 3900], [0.03, 0.02, 0.012, 0.008], [1.0, 0.6, 0.35, 0.2], 0.3, r, 0.03), 0.0)
    place(x, modal([160], [0.04], [0.7], 0.2, r), 0.0)
    place(x, click(r, 0.01, 1500, 8000, 0.0015), 0.0, 0.6)
    return room(x, r, 0.2, 0.1)


@som("interagir_celular")
def _(r):
    x = silence(0.3)
    place(x, modal([1250, 2600, 4100], [0.015, 0.009, 0.005], [1.0, 0.5, 0.25], 0.15, r, 0.03), 0.0)
    place(x, modal([260], [0.02], [0.6], 0.1, r), 0.0)
    place(x, modal([1250, 2600], [0.012, 0.007], [0.4, 0.2], 0.1, r, 0.03), 0.07)
    return room(x, r, 0.18, 0.08)


@som("interagir_midia")
def _(r):
    x = silence(0.3)
    place(x, modal([2300, 3900, 6100], [0.03, 0.02, 0.01], [1.0, 0.5, 0.2], 0.2, r, 0.02), 0.0)
    place(x, modal([420, 760], [0.03, 0.02], [0.6, 0.3], 0.15, r), 0.0)
    return room(x, r, 0.18, 0.08)


@som("interagir_ferramenta")
def _(r):
    x = silence(0.4)
    place(x, modal([560, 1080, 1900], [0.05, 0.03, 0.02], [1.0, 0.5, 0.25], 0.3, r, 0.03), 0.0)
    place(x, grains(0.2, r, 200, 1500, 6000, amp=(0.1, 0.3)), 0.02, 0.5)
    return room(x, r, 0.2, 0.1)


# --- ações periciais

@som("foto")
def _(r):
    x = silence(1.1)
    place(x, click(r, 0.01, 1500, 9000, 0.0015), 0.0, 0.9)  # 1ª cortina
    place(x, modal([310, 720, 1500], [0.03, 0.02, 0.01], [1.0, 0.6, 0.3], 0.1, r) + band(r.standard_normal(int(0.1 * SR)), 300, 3000) * env_exp(int(0.1 * SR), 0.012) * 0.5, 0.012, 0.8)  # espelho
    place(x, click(r, 0.01, 1500, 9000, 0.0015), 0.07, 0.8)  # 2ª cortina
    place(x, modal([380, 900], [0.02, 0.01], [0.6, 0.3], 0.06, r), 0.075, 0.6)
    w = sweep(2400, 7800, 0.65) * env_ar(int(0.65 * SR), 0.05, 0.25)
    place(x, w, 0.3, 0.035)  # recarga do flash
    return room(x, r, 0.22, 0.12)


@som("papel")
def _(r):
    shape = lambda u: 0.4 + 0.6 * np.sin(np.pi * u)
    x = grains(0.9, r, 70, 1800, 9000, glen=(0.003, 0.025), amp=(0.1, 1.0), shape=shape)
    x += band(r.standard_normal(len(x)), 2000, 7000, 2) * 0.03 * np.sin(np.pi * np.linspace(0, 1, len(x)))
    return room(x, r, 0.2, 0.12)


def fita(r, dur, rate0, rate1, lo=900, hi=7000):
    """Fita adesiva sendo puxada: estalos regulares de descolamento."""
    x = silence(dur + 0.05)
    t, k = 0.0, 0
    while t < dur:
        u = t / dur
        rate = rate0 + (rate1 - rate0) * u
        g = click(r, 0.004, lo, hi, 0.0009)
        place(x, g, t, r.uniform(0.5, 1.0) * (0.6 + 0.4 * np.sin(np.pi * u)))
        t += 1.0 / rate * r.uniform(0.85, 1.15)
        k += 1
    x += band(r.standard_normal(len(x)), 1500, 6000, 2) * 0.06
    return x


@som("deslacrar")
def _(r):
    x = silence(0.9)
    place(x, fita(r, 0.45, 160, 420), 0.05, 1.0)
    place(x, modal([1400, 2900], [0.01, 0.006], [1.0, 0.5], 0.05, r) + click(r, 0.01, 1500, 8000, 0.002), 0.52, 0.9)  # rompe
    place(x, grains(0.25, r, 300, 2000, 8000, amp=(0.1, 0.4)), 0.56, 0.5)
    return room(x, r, 0.2, 0.12)


@som("lacre_aplicar")
def _(r):
    x = silence(1.3)
    place(x, fita(r, 0.6, 300, 220), 0.0, 0.9)
    place(x, click(r, 0.02, 1000, 6000, 0.003), 0.65, 0.6)  # corte
    for k in range(3):  # alisar com o polegar
        place(x, band(r.standard_normal(int(0.12 * SR)), 300, 2500, 2) * env_ar(int(0.12 * SR), 0.03, 0.06), 0.8 + k * 0.14, 0.35)
    place(x, modal([180, 330], [0.04, 0.02], [0.7, 0.3], 0.15, r), 0.8, 0.5)
    return room(x, r, 0.2, 0.12)


def parafuso(r, dur=0.55):
    x = silence(dur)
    t = 0.0
    while t < dur - 0.05:
        place(x, click(r, 0.006, 2500, 9000, 0.001), t, r.uniform(0.3, 0.6))
        place(x, modal([3100, 5200], [0.008, 0.004], [0.3, 0.15], 0.03, r, 0.05), t, 0.4)
        t += r.uniform(0.045, 0.07)
    x += band(r.standard_normal(len(x)), 2000, 6000, 2) * 0.04 * env_ar(len(x), 0.05, 0.1)
    return x


@som("abrir_gabinete")
def _(r):
    x = silence(2.4)
    place(x, parafuso(r), 0.05, 0.8)
    place(x, modal([1800, 3600], [0.02, 0.01], [0.5, 0.2], 0.05, r), 0.62, 0.5)
    place(x, parafuso(r), 0.8, 0.8)
    n = int(0.55 * SR)
    slide = band(r.standard_normal(n), 400, 2500, 2) * env_ar(n, 0.08, 0.2) * (0.7 + 0.3 * np.sin(np.linspace(0, 9, n)))
    place(x, slide, 1.5, 0.5)
    place(x, modal([210, 470, 880], [0.12, 0.08, 0.05], [1.0, 0.6, 0.35], 0.4, r), 2.05, 0.9)  # tampa apoiada
    return room(x, r, 0.24, 0.12)


@som("componente_remover")
def _(r):
    x = silence(0.8)
    place(x, modal([1500, 2800], [0.015, 0.008], [1.0, 0.5], 0.06, r) + click(r, 0.01, 1000, 7000, 0.0015), 0.0, 1.0)  # trava
    place(x, band(r.standard_normal(int(0.25 * SR)), 1200, 5000, 2) * env_ar(int(0.25 * SR), 0.05, 0.1), 0.12, 0.25)
    place(x, modal([900, 1900, 3300], [0.1, 0.06, 0.03], [0.8, 0.5, 0.3], 0.35, r, 0.03), 0.42, 0.8)  # dissipador na manta
    return room(x, r, 0.22, 0.12)


@som("midia_remover")
def _(r):
    x = silence(0.9)
    n = int(0.12 * SR)
    place(x, band(r.standard_normal(n), 1500, 6000, 2) * env_ar(n, 0.02, 0.05), 0.0, 0.4)  # atrito do conector
    place(x, click(r, 0.012, 1200, 6000, 0.002) + modal([1600], [0.01], [0.6], 0.03, r), 0.12, 0.9)  # solta
    place(x, band(r.standard_normal(int(0.3 * SR)), 500, 3000, 2) * env_ar(int(0.3 * SR), 0.05, 0.15), 0.25, 0.35)  # desliza no trilho
    place(x, modal([520, 1150, 2300], [0.06, 0.04, 0.02], [1.0, 0.5, 0.3], 0.3, r), 0.58, 0.8)  # apoia na bancada
    return room(x, r, 0.22, 0.12)


def bip(f, dur=0.08, level=1.0):
    return tone(f, dur, harm=(1.0, 0.0, 0.12), att=0.002, rel=0.01) * level


@som("bloqueio")
def _(r):
    x = silence(0.9)
    place(x, band(r.standard_normal(int(0.06 * SR)), 1500, 7000, 2) * env_ar(int(0.06 * SR), 0.01, 0.03), 0.0, 0.4)
    place(x, click(r, 0.012, 1500, 8000, 0.0015) + modal([2100, 3700], [0.01, 0.006], [0.5, 0.2], 0.03, r), 0.06, 1.0)
    place(x, bip(2093, 0.07), 0.35, 0.25)
    place(x, bip(2637, 0.09), 0.45, 0.25)
    return room(x, r, 0.2, 0.08)


@som("aquisicao")
def _(r):
    dur = 3.4
    x = silence(dur)
    t = t_axis(2.9)
    # motor girando (sobe de rotação) + ventoinha do duplicador
    f = 40 + 80 * (1 - np.exp(-t / 0.5))
    ph = 2 * np.pi * np.cumsum(f) / SR
    hum = (np.sin(ph) + 0.5 * np.sin(2 * ph) + 0.25 * np.sin(3 * ph)) * env_ar(len(t), 0.3, 0.4)
    fan = band(r.standard_normal(len(t)), 300, 2500, 2) * env_ar(len(t), 0.5, 0.4)
    place(x, hum, 0.0, 0.25)
    place(x, fan, 0.0, 0.12)
    tt = 0.35
    while tt < 2.7:  # cabeça buscando trilhas
        place(x, modal([1900, 3300], [0.004, 0.003], [1.0, 0.5], 0.02, r, 0.1) + click(r, 0.004, 2000, 8000, 0.0008) * 0.5, tt, r.uniform(0.15, 0.45))
        tt += r.exponential(0.06)
    place(x, bip(1568, 0.08), 2.95, 0.3)
    place(x, bip(2093, 0.12), 3.07, 0.3)
    return room(x, r, 0.2, 0.08)


@som("hash")
def _(r):
    x = silence(1.0)
    tt = 0.0
    while tt < 0.55:
        place(x, modal([1900, 3300], [0.004, 0.003], [1.0, 0.5], 0.02, r, 0.1), tt, r.uniform(0.1, 0.3))
        tt += r.exponential(0.05)
    place(x, bip(1760, 0.06), 0.62, 0.35)
    place(x, bip(1760, 0.06), 0.72, 0.35)
    return room(x, r, 0.2, 0.06)


@som("energia_desligar")
def _(r):
    x = silence(2.0)
    place(x, click(r, 0.012, 1000, 6000, 0.002) + modal([1300, 2500], [0.012, 0.006], [0.6, 0.3], 0.03, r), 0.0, 0.9)
    n = int(1.8 * SR)
    tt = np.arange(n) / SR
    f = 130 * np.exp(-tt / 0.6) + 8
    ph = 2 * np.pi * np.cumsum(f) / SR
    whine = (np.sin(ph) + 0.3 * np.sin(3 * ph)) * np.exp(-tt / 0.7)
    air = np.zeros(n)
    nz = r.standard_normal(n)
    for k in range(0, n, 2048):  # filtro passa-baixas com corte descendo
        seg = nz[k:k + 2048]
        fc = max(150, 4000 * np.exp(-(k / SR) / 0.5))
        air[k:k + len(seg)] = band(seg, hi=fc, order=2)
    air *= np.exp(-tt / 0.6)
    place(x, whine, 0.05, 0.35)
    place(x, air, 0.05, 0.4)
    return room(x, r, 0.22, 0.1)


@som("rede_isolar")
def _(r):
    x = silence(0.6)
    place(x, modal([2200, 3800], [0.008, 0.005], [1.0, 0.5], 0.03, r) + click(r, 0.008, 1500, 8000, 0.0012), 0.0, 0.8)  # trava do RJ45
    n = int(0.15 * SR)
    place(x, band(r.standard_normal(n), 900, 4000, 2) * env_ar(n, 0.02, 0.08), 0.06, 0.35)
    place(x, modal([600, 1300], [0.02, 0.012], [0.6, 0.3], 0.08, r), 0.24, 0.5)  # cabo solto bate
    return room(x, r, 0.2, 0.1)


def tecla(r):
    return modal([1900 * r.uniform(0.85, 1.15), 3600, 520], [0.008, 0.004, 0.012], [0.6, 0.25, 0.5], 0.06, r, 0.05) + click(r, 0.008, 1500, 8000, 0.0015) * 0.7


for _k in range(3):
    SONS[f"tecla_{_k + 1}"] = ((lambda k: lambda r: room(tecla(r), r, 0.15, 0.05))(_k), 0.35)


@som("digitacao")
def _(r):
    x = silence(1.7)
    tt = 0.02
    while tt < 1.15:
        place(x, tecla(r), tt, r.uniform(0.5, 1.0))
        tt += r.uniform(0.07, 0.16)
    place(x, tecla(r) * 1.3, 1.25, 1.0)  # Enter
    place(x, bip(1760, 0.07), 1.45, 0.2)
    return room(x, r, 0.2, 0.08)


@som("faraday")
def _(r):
    x = silence(1.3)
    # velcro: estalos densos de alta frequência com envelope de rasgo
    n = int(0.55 * SR)
    cr = r.standard_normal(n) * (r.random(n) < 0.25)
    cr = band(cr, 1500, 10000, 2) * env_ar(n, 0.03, 0.15) * (0.6 + 0.4 * np.abs(np.sin(np.linspace(0, 40, n))))
    place(x, tecido(r, 0.5, 0.8), 0.0, 0.6)
    place(x, cr, 0.55, 0.9)
    return room(x, r, 0.2, 0.1)


@som("aluminio")
def _(r):
    shape = lambda u: 0.5 + 0.5 * np.sin(np.pi * u)
    x = grains(1.3, r, 75, 2500, 12000, glen=(0.0008, 0.005), amp=(0.1, 1.0), shape=shape)
    x += grains(1.3, r, 18, 600, 3000, glen=(0.01, 0.03), amp=(0.1, 0.3), shape=shape)
    return room(x, r, 0.2, 0.1)


def toque_tela(r):
    return click(r, 0.006, 1500, 6000, 0.001) * 0.6 + modal([260], [0.01], [0.4], 0.03, r)


def vibra(dur=0.12):
    t = t_axis(dur)
    return np.sin(2 * np.pi * 170 * t) * np.sign(np.sin(2 * np.pi * 170 * t)) ** 2 * env_ar(len(t), 0.01, 0.03)


@som("celular_toque", peak=0.6)
def _(r):
    x = silence(0.55)
    place(x, toque_tela(r), 0.0)
    place(x, toque_tela(r), 0.16, 0.8)
    place(x, vibra(0.09), 0.19, 0.35)
    return room(x, r, 0.15, 0.05)


@som("celular_carregar")
def _(r):
    x = silence(1.0)
    place(x, band(r.standard_normal(int(0.05 * SR)), 1500, 6000, 2) * env_ar(int(0.05 * SR), 0.01, 0.02), 0.0, 0.4)
    place(x, click(r, 0.01, 1500, 8000, 0.0015) + modal([2400], [0.008], [0.5], 0.02, r), 0.05, 0.9)
    place(x, vibra(0.12), 0.32, 0.4)
    place(x, chime([1047, 1568], 0.07, 0.02), 0.36, 0.2)
    return room(x, r, 0.18, 0.08)


@som("sim_remover")
def _(r):
    x = silence(0.7)
    place(x, modal([3800, 6200], [0.006, 0.004], [1.0, 0.4], 0.02, r) + click(r, 0.006, 2500, 9000, 0.001), 0.0, 0.8)  # agulha
    place(x, modal([2600, 4300], [0.01, 0.006], [0.8, 0.4], 0.03, r), 0.12, 0.9)  # bandeja solta
    n = int(0.22 * SR)
    place(x, band(r.standard_normal(n), 2500, 8000, 2) * env_ar(n, 0.02, 0.08), 0.16, 0.25)  # desliza
    place(x, modal([3100, 5200], [0.015, 0.008], [0.6, 0.3], 0.05, r), 0.45, 0.6)  # chip na bandeja
    return room(x, r, 0.18, 0.08)


@som("extracao")
def _(r):
    x = silence(2.2)
    place(x, click(r, 0.012, 1500, 8000, 0.0015) + modal([2100], [0.01], [0.5], 0.03, r), 0.0, 0.9)
    place(x, vibra(0.1), 0.25, 0.4)
    for k in range(6):
        place(x, bip(1319 + 60 * k, 0.04), 0.5 + k * 0.2, 0.15)
    place(x, chime([1319, 1760], 0.09, 0.03), 1.75, 0.3)
    return room(x, r, 0.18, 0.06)


@som("embalar")
def _(r):
    shape = lambda u: 0.3 + 0.7 * np.sin(np.pi * u) ** 0.5
    x = grains(1.4, r, 95, 900, 8000, glen=(0.006, 0.035), amp=(0.1, 0.8), shape=shape)
    n = int(0.25 * SR)
    zip_ = sum(click(r, 0.004, 2000, 8000, 0.0008) for _ in range(1))
    z = silence(0.3)
    for k in range(18):  # zip
        place(z, click(r, 0.004, 2500, 9000, 0.0007), k * 0.014, r.uniform(0.4, 0.8))
    place(x, z, 1.1, 0.7)
    return room(x, r, 0.2, 0.1)


@som("etiquetar")
def _(r):
    x = silence(0.8)
    place(x, fita(r, 0.25, 500, 900, 2000, 10000), 0.0, 0.7)  # descola a etiqueta
    for k in range(2):
        place(x, band(r.standard_normal(int(0.1 * SR)), 300, 2500, 2) * env_ar(int(0.1 * SR), 0.02, 0.05), 0.38 + k * 0.13, 0.35)
    place(x, modal([200, 380], [0.03, 0.015], [0.6, 0.3], 0.1, r), 0.38, 0.4)
    return room(x, r, 0.18, 0.1)


@som("inspecionar")
def _(r):
    x = silence(1.8)
    tt = 0.05
    for k in range(7):
        kind = r.integers(3)
        if kind == 0:
            g = modal([r.uniform(300, 700), r.uniform(900, 1800)], [0.04, 0.02], [1.0, 0.4], 0.15, r)
        elif kind == 1:
            g = grains(0.2, r, 200, 1500, 7000, amp=(0.2, 0.6))
        else:
            g = band(r.standard_normal(int(0.15 * SR)), 300, 2000, 2) * env_ar(int(0.15 * SR), 0.03, 0.08)
        place(x, g, tt, r.uniform(0.3, 0.8))
        tt += r.uniform(0.12, 0.28)
    return room(x, r, 0.24, 0.14)


@som("caneta")
def _(r):
    x = silence(1.2)
    tt = 0.05
    while tt < 1.05:
        n = int(r.uniform(0.04, 0.12) * SR)
        place(x, band(r.standard_normal(n), 2500, 7000, 2) * env_ar(n, 0.01, 0.02), tt, r.uniform(0.3, 0.8))
        tt += n / SR + r.uniform(0.01, 0.06)
    return room(x, r, 0.18, 0.08)


# --- ambientes (estéreo, em loop sem emenda)

def loop_stereo(gen, dur, r, xfade=2.0):
    """Gera dur+xfade segundos e funde o final no início (loop contínuo)."""
    L, R = gen(dur + xfade, r)
    n, f = int(dur * SR), int(xfade * SR)
    ramp = np.linspace(0, 1, f)
    out = []
    for ch in (L, R):
        y = ch[:n].copy()
        y[:f] = y[:f] * np.sqrt(ramp) + ch[n:n + f] * np.sqrt(1 - ramp)
        out.append(y)
    return np.array(out)


def amb_lab(dur, r):
    n = int(dur * SR)
    t = np.arange(n) / SR
    chans = []
    for c in range(2):
        hvac = band(pink(n, r), 40, 700, 2) * (0.85 + 0.15 * np.sin(2 * np.pi * t / 7.3 + c))
        hiss = band(r.standard_normal(n), 2000, 6000, 2) * 0.006
        chans.append(hvac * 0.5 + hiss)
    hum = sum(a * np.sin(2 * np.pi * f * t) for f, a in [(120, 1.0), (240, 0.45), (360, 0.2), (480, 0.08)])
    buzz = band(np.sign(np.sin(2 * np.pi * 120 * t)) * 0.3, 1500, 5000, 2)  # reator da luminária
    fans = band(r.standard_normal(n), 700, 1400, 2) * 0.025 + 0.003 * np.sin(2 * np.pi * 1230 * t)
    L = chans[0] + hum * 0.035 + buzz * 0.02 + fans * 0.7
    R = chans[1] + hum * 0.03 + buzz * 0.025 + fans * 1.0
    # sons distantes: porta, passos no corredor (abafados)
    for k in range(3):
        at = r.uniform(2, dur - 3)
        ev = room(band(modal([90, 160, 300], [0.15, 0.1, 0.05], [1.0, 0.5, 0.2], 0.6, r), hi=900, order=2), r, 0.6, 0.6, 1500)
        g = r.uniform(0.04, 0.08)
        pan = r.uniform(0.3, 0.7)
        place(L, ev, at, g * (1 - pan) * 2)
        place(R, ev, at, g * pan * 2)
    return L, R


def amb_escritorio(dur, r):
    n = int(dur * SR)
    t = np.arange(n) / SR
    L = band(pink(n, r), 40, 500, 2) * 0.35
    R = band(pink(n, r), 40, 500, 2) * 0.35
    # trânsito pela janela (lado direito): passagens de carros com efeito Doppler simples
    traffic = band(brown(n, r), 30, 600, 2) * 0.25
    for k in range(int(dur / 6)):
        at = r.uniform(0, dur - 4)
        m = int(4 * SR)
        car = band(r.standard_normal(m), 80, 1800, 2) * signal.windows.gaussian(m, m / 7)
        place(traffic, car, at, r.uniform(0.15, 0.3))
    traffic = band(traffic, hi=1500, order=2)
    L += traffic * 0.45
    R += traffic * 1.0
    # pássaros distantes
    for k in range(int(dur / 5)):
        at = r.uniform(0, dur - 1)
        f0 = r.uniform(2800, 4200)
        for j in range(r.integers(2, 5)):
            ch = sweep(f0, f0 * r.uniform(1.1, 1.4), 0.08) * env_ar(int(0.08 * SR), 0.01, 0.04)
            place(R, ch, at + j * 0.12, 0.012)
            place(L, ch, at + j * 0.12, 0.005)
    # relógio de parede (1 tique por segundo, no lado esquerdo)
    tick = modal([2600, 4100], [0.004, 0.003], [1.0, 0.4], 0.03, r)
    for s in range(int(dur)):
        place(L, tick, s + 0.25, 0.025)
        place(R, tick, s + 0.25, 0.01)
    hum = 0.012 * (np.sin(2 * np.pi * 120 * t) + 0.4 * np.sin(2 * np.pi * 240 * t))
    return L + hum, R + hum


def amb_ventoinha(dur, r):
    n = int(dur * SR)
    t = np.arange(n) / SR
    chans = []
    for c in range(2):
        x = band(r.standard_normal(n), 200, 3000, 2) * 0.3 + band(pink(n, r), 50, 400, 2) * 0.3
        x += 0.05 * np.sin(2 * np.pi * 186 * t) + 0.02 * np.sin(2 * np.pi * 372 * t)  # pás
        chans.append(x)
    return chans[0], chans[1]


AMBIENTES = {
    "ambiente_laboratorio": (amb_lab, 24.0, 0.30),
    "ambiente_escritorio": (amb_escritorio, 30.0, 0.32),
    "ambiente_estacao": (amb_ventoinha, 6.0, 0.25),
}


def main():
    out = os.path.join(sys.argv[1] if len(sys.argv) > 1 else "assets", "sounds")
    os.makedirs(out, exist_ok=True)
    total = 0.0
    for k, (nome, (fn, peak)) in enumerate(sorted(SONS.items())):
        r = rng(1000 + k * 7919 + sum(map(ord, nome)))
        x = finish(fn(r), peak)
        write(os.path.join(out, nome + ".wav"), x)
        total += len(x) / SR
        print(f"som {nome:24s} {len(x) / SR:5.2f} s")
    for k, (nome, (fn, dur, peak)) in enumerate(AMBIENTES.items()):
        r = rng(5000 + k)
        x = loop_stereo(fn, dur, r)
        x = x / np.abs(x).max() * peak
        write(os.path.join(out, nome + ".wav"), x)
        total += dur
        print(f"ambiente {nome:19s} {dur:5.2f} s (loop)")
    print(f"{len(SONS) + len(AMBIENTES)} arquivos, {total:.1f} s de áudio")


if __name__ == "__main__":
    main()
