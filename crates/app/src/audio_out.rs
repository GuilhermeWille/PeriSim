//! Saída de som pelo cpal — Windows (WASAPI) e macOS (CoreAudio).
//!
//! O mixer do jogo produz estéreo i16 intercalado a `rate` Hz; o dispositivo pode pedir
//! outra taxa (48 kHz é comum no Windows), outro formato (f32, i16…) e outro número de
//! canais. Aqui se faz a conversão: reamostragem linear e cópia do estéreo para os dois
//! primeiros canais (mono recebe a média; canais extras ficam em silêncio).

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

const CHUNK: usize = 512;

/// Reamostrador linear com um quadro de atraso (não precisa olhar adiante).
struct Rs {
    buf: Vec<i16>,
    n: usize,
    pos: f64,
    step: f64,
    prev: [f32; 2],
}

impl Rs {
    fn new(step: f64) -> Rs {
        Rs { buf: vec![0; CHUNK * 2], n: 0, pos: 0.0, step, prev: [0.0; 2] }
    }

    fn frame(&self, i: usize) -> [f32; 2] {
        [self.buf[i * 2] as f32 / 32768.0, self.buf[i * 2 + 1] as f32 / 32768.0]
    }

    fn next(&mut self, render: &mut impl FnMut(&mut [i16], usize)) -> [f32; 2] {
        while self.pos >= self.n as f64 {
            if self.n > 0 {
                self.prev = self.frame(self.n - 1);
            }
            self.pos -= self.n as f64;
            render(&mut self.buf, CHUNK);
            self.n = CHUNK;
        }
        let i = self.pos.floor() as usize;
        let f = (self.pos - i as f64) as f32;
        let a = if i == 0 { self.prev } else { self.frame(i - 1) };
        let b = self.frame(i);
        self.pos += self.step;
        [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
    }
}

fn build<T>(dev: &cpal::Device, cfg: cpal::StreamConfig, rate: u32, mut render: impl FnMut(&mut [i16], usize) + Send + 'static) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let ch = (cfg.channels as usize).max(1);
    let mut rs = Rs::new(rate as f64 / cfg.sample_rate as f64);
    dev.build_output_stream(
        cfg,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for fr in data.chunks_mut(ch) {
                let [l, r] = rs.next(&mut render);
                if ch == 1 {
                    fr[0] = T::from_sample((l + r) * 0.5);
                    continue;
                }
                fr[0] = T::from_sample(l);
                fr[1] = T::from_sample(r);
                for s in fr.iter_mut().skip(2) {
                    *s = T::EQUILIBRIUM;
                }
            }
        },
        |e| eprintln!("[áudio] erro na saída: {}", e),
        None,
    )
    .map_err(|e| e.to_string())
}

/// Abre a saída padrão do sistema e mantém o fluxo vivo (bloqueia a thread chamadora).
/// `render(buf, quadros)` preenche `buf` com estéreo i16 a `rate` Hz; `ready` recebe uma
/// descrição do dispositivo quando o som começa. Sem dispositivo, retorna o motivo.
pub fn run(rate: u32, render: impl FnMut(&mut [i16], usize) + Send + 'static, ready: impl FnOnce(String)) -> Result<(), String> {
    let host = cpal::default_host();
    let dev = host.default_output_device().ok_or_else(|| "nenhum dispositivo de saída de som".to_string())?;
    let sup = dev.default_output_config().map_err(|e| e.to_string())?;
    let fmt = sup.sample_format();
    let cfg = sup.config();
    let info = format!("{} Hz, {} canal(is), {}", cfg.sample_rate, cfg.channels, fmt);
    let stream = match fmt {
        SampleFormat::F32 => build::<f32>(&dev, cfg, rate, render)?,
        SampleFormat::I16 => build::<i16>(&dev, cfg, rate, render)?,
        SampleFormat::U16 => build::<u16>(&dev, cfg, rate, render)?,
        SampleFormat::I32 => build::<i32>(&dev, cfg, rate, render)?,
        SampleFormat::F64 => build::<f64>(&dev, cfg, rate, render)?,
        other => return Err(format!("formato de amostra {} não suportado", other)),
    };
    stream.play().map_err(|e| e.to_string())?;
    ready(info);
    // o fluxo para quando é descartado: esta thread só o mantém vivo
    loop {
        std::thread::park();
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn reamostragem_preserva_tom_e_continuidade() {
        // 44,1 kHz → 48 kHz: rampa contínua continua contínua e na mesma inclinação
        let mut k: i32 = 0;
        let mut src = |b: &mut [i16], n: usize| {
            for i in 0..n {
                let v = (k % 20000) as i16;
                b[i * 2] = v;
                b[i * 2 + 1] = -v;
                k += 1;
            }
        };
        let mut rs = Rs::new(44100.0 / 48000.0);
        let out: Vec<[f32; 2]> = (0..5000).map(|_| rs.next(&mut src)).collect();
        for w in out.windows(2).skip(2) {
            let d = (w[1][0] - w[0][0]) * 32768.0;
            assert!((d - 44100.0 / 48000.0).abs() < 0.02, "passo {}", d);
            assert!((w[1][0] + w[1][1]).abs() < 1e-6);
        }
    }
}
