//! Áudio do jogo: mixer próprio (em uma thread), sons em `assets/sounds/*.wav`.
//!
//! Saídas:
//!  * Linux: ALSA carregada em tempo de execução (`dlopen("libasound.so.2")`). Se a
//!    biblioteca ou o dispositivo não existirem (VM sem placa de som, sessão remota),
//!    o jogo segue em silêncio e o menu de configurações mostra o motivo — nunca aborta.
//!  * `PERICIA_AUDIO_WAV=<arquivo>`: grava a mixagem num WAV em tempo real (validação
//!    sem placa de som e vídeos de demonstração com som).
//!  * `PERICIA_AUDIO=off`: sem áudio.
//!  * Outras plataformas: ainda sem saída (silêncio), informado no menu.
//!
//! Categorias com volume próprio: ambiente, efeitos (objetos, passos, ações) e interface.

use macroquad::prelude::Vec3;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const RATE: u32 = 44100;
const CHUNK: usize = 1024;
const MAX_VOICES: usize = 40;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cat {
    Ambiente = 0,
    Efeitos = 1,
    Interface = 2,
}

pub struct Clip {
    /// Amostras intercaladas (1 ou 2 canais), em 44,1 kHz.
    pub data: Vec<f32>,
    pub ch: usize,
}

impl Clip {
    pub fn frames(&self) -> usize {
        self.data.len() / self.ch
    }
}

struct Voice {
    clip: Arc<Clip>,
    pos: f64,
    rate: f64,
    gl: f32,
    gr: f32,
    cat: Cat,
    looping: bool,
    id: u64,
    fade: f32,
    fade_to: f32,
    fade_step: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volumes {
    pub master: f32,
    pub cat: [f32; 3],
    pub muted: bool,
}

impl Default for Volumes {
    fn default() -> Self {
        Volumes { master: 0.8, cat: [0.55, 0.9, 0.6], muted: false }
    }
}

pub struct Mix {
    voices: Vec<Voice>,
    vol: Volumes,
    /// volume efetivo suavizado por categoria (evita estalos ao mexer no controle)
    cur: [f32; 3],
}

impl Mix {
    pub fn new() -> Mix {
        Mix { voices: vec![], vol: Volumes::default(), cur: [0.0; 3] }
    }

    /// Mistura `frames` quadros estéreo em `out` (intercalado, i16).
    pub fn render(&mut self, out: &mut [i16], frames: usize) {
        let mut acc = vec![0.0f32; frames * 2];
        let target: [f32; 3] = std::array::from_fn(|i| if self.vol.muted { 0.0 } else { self.vol.master * self.vol.cat[i] });
        let start = self.cur;
        for v in self.voices.iter_mut() {
            let n = v.clip.frames();
            if n == 0 {
                v.fade = 0.0;
                v.fade_to = 0.0;
                continue;
            }
            let c = v.cat as usize;
            for f in 0..frames {
                let u = f as f32 / frames as f32;
                let cv = start[c] + (target[c] - start[c]) * u;
                if v.fade != v.fade_to {
                    v.fade = if v.fade < v.fade_to { (v.fade + v.fade_step).min(v.fade_to) } else { (v.fade - v.fade_step).max(v.fade_to) };
                }
                let mut p = v.pos;
                if p >= n as f64 {
                    if v.looping {
                        p %= n as f64;
                    } else {
                        break;
                    }
                }
                let i0 = p as usize;
                let i1 = if i0 + 1 < n { i0 + 1 } else if v.looping { 0 } else { i0 };
                let fr = (p - i0 as f64) as f32;
                let (l, r) = if v.clip.ch == 2 {
                    let d = &v.clip.data;
                    (d[i0 * 2] + (d[i1 * 2] - d[i0 * 2]) * fr, d[i0 * 2 + 1] + (d[i1 * 2 + 1] - d[i0 * 2 + 1]) * fr)
                } else {
                    let d = &v.clip.data;
                    let m = d[i0] + (d[i1] - d[i0]) * fr;
                    (m, m)
                };
                let g = cv * v.fade;
                acc[f * 2] += l * v.gl * g;
                acc[f * 2 + 1] += r * v.gr * g;
                v.pos = p + v.rate;
            }
        }
        self.cur = target;
        self.voices.retain(|v| (v.looping || v.pos < v.clip.frames() as f64) && !(v.fade_to == 0.0 && v.fade <= 0.0));
        for (o, s) in out.iter_mut().zip(acc) {
            // limitador suave: acima de 0,8 comprime em direção a 1,0
            let a = s.abs();
            let y = if a > 0.8 { s.signum() * (0.8 + 0.2 * ((a - 0.8) / 0.2).tanh()) } else { s };
            *o = (y.clamp(-1.0, 1.0) * 32767.0) as i16;
        }
    }
}

pub struct Audio {
    mix: Arc<Mutex<Mix>>,
    clips: HashMap<String, Arc<Clip>>,
    pub status: Arc<Mutex<String>>,
    pub ok: Arc<AtomicBool>,
    next: u64,
    ambient: Option<(String, u64)>,
    layer: Option<(String, u64)>,
    seed: u32,
    rec: Option<(Arc<AtomicBool>, std::thread::JoinHandle<()>)>,
    /// Mixagem presa aos quadros (vídeos de demonstração): (arquivo, amostras, fração pendente).
    offline: Option<(String, Vec<i16>, f64)>,
    /// Último som tocado por nome (instante), para não empilhar o mesmo som no mesmo quadro.
    last: HashMap<String, f64>,
    /// Registro dos sons tocados (roteiros de teste conferem a trilha).
    pub log: Vec<(f64, String)>,
}

/// Lê um WAV PCM 16 bits (1–2 canais). Outras taxas são reamostradas para 44,1 kHz.
pub fn parse_wav(b: &[u8]) -> Result<Clip, String> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("não é RIFF/WAVE".into());
    }
    let (mut ch, mut rate, mut bits, mut fmt) = (0usize, 0u32, 0u16, 0u16);
    let mut data: Option<&[u8]> = None;
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let sz = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
        let body = &b[i + 8..(i + 8 + sz).min(b.len())];
        if id == b"fmt " && body.len() >= 16 {
            fmt = u16::from_le_bytes([body[0], body[1]]);
            ch = u16::from_le_bytes([body[2], body[3]]) as usize;
            rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            data = Some(body);
        }
        i += 8 + sz + (sz & 1);
    }
    if fmt != 1 || bits != 16 || !(1..=2).contains(&ch) || rate == 0 {
        return Err(format!("formato não suportado (fmt {}, {} bits, {} canais)", fmt, bits, ch));
    }
    let d = data.ok_or("sem bloco de dados")?;
    let mut v: Vec<f32> = d.chunks_exact(2).map(|x| i16::from_le_bytes([x[0], x[1]]) as f32 / 32768.0).collect();
    if rate != RATE {
        let n = v.len() / ch;
        let m = (n as f64 * RATE as f64 / rate as f64) as usize;
        let mut w = Vec::with_capacity(m * ch);
        for k in 0..m {
            let p = k as f64 * rate as f64 / RATE as f64;
            let i0 = (p as usize).min(n - 1);
            let i1 = (i0 + 1).min(n - 1);
            let f = (p - i0 as f64) as f32;
            for c in 0..ch {
                w.push(v[i0 * ch + c] + (v[i1 * ch + c] - v[i0 * ch + c]) * f);
            }
        }
        v = w;
    }
    Ok(Clip { data: v, ch })
}

pub fn wav_bytes(samples: &[i16], ch: u16) -> Vec<u8> {
    let mut o = Vec::with_capacity(44 + samples.len() * 2);
    let data_len = (samples.len() * 2) as u32;
    o.extend_from_slice(b"RIFF");
    o.extend_from_slice(&(36 + data_len).to_le_bytes());
    o.extend_from_slice(b"WAVEfmt ");
    o.extend_from_slice(&16u32.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&ch.to_le_bytes());
    o.extend_from_slice(&RATE.to_le_bytes());
    o.extend_from_slice(&(RATE * ch as u32 * 2).to_le_bytes());
    o.extend_from_slice(&(ch * 2).to_le_bytes());
    o.extend_from_slice(&16u16.to_le_bytes());
    o.extend_from_slice(b"data");
    o.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        o.extend_from_slice(&s.to_le_bytes());
    }
    o
}

impl Audio {
    /// Carrega os sons e inicia a saída. Nunca falha: sem saída, fica em silêncio.
    pub fn new(vol: Volumes) -> Audio {
        let mix = Arc::new(Mutex::new(Mix::new()));
        mix.lock().unwrap().vol = vol;
        let status = Arc::new(Mutex::new(String::from("iniciando…")));
        let ok = Arc::new(AtomicBool::new(false));
        let mut a = Audio { mix, clips: HashMap::new(), status, ok, next: 1, ambient: None, layer: None, seed: 0x9e37_79b9, rec: None, offline: None, last: HashMap::new(), log: vec![] };
        let loaded = a.load_clips();
        if std::env::var("PERICIA_AUDIO").map(|v| v == "off").unwrap_or(false) {
            a.set_status("desativado (PERICIA_AUDIO=off)");
        } else if let Ok(path) = std::env::var("PERICIA_AUDIO_OFFLINE") {
            a.ok.store(true, Ordering::SeqCst);
            a.set_status(&format!("mixagem por quadro em {}", path));
            a.offline = Some((path, vec![], 0.0));
        } else if let Ok(path) = std::env::var("PERICIA_AUDIO_WAV") {
            a.start_recorder(path);
        } else {
            a.start_device();
        }
        if loaded == 0 {
            let s = a.status.lock().unwrap().clone();
            a.set_status(&format!("{} — nenhum som encontrado em assets/sounds", s));
        }
        a
    }

    fn set_status(&self, s: &str) {
        *self.status.lock().unwrap() = s.to_string();
    }

    fn load_clips(&mut self) -> usize {
        let Some(dir) = crate::assets::assets_dir().map(|d| d.join("sounds")) else { return 0 };
        let Ok(rd) = std::fs::read_dir(&dir) else { return 0 };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("wav") {
                continue;
            }
            let name = p.file_stem().and_then(|x| x.to_str()).unwrap_or("").to_string();
            match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|b| parse_wav(&b)) {
                Ok(c) => {
                    self.clips.insert(name, Arc::new(c));
                }
                Err(e) => eprintln!("[áudio] {}: {}", p.display(), e),
            }
        }
        self.clips.len()
    }

    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    pub fn has(&self, name: &str) -> bool {
        self.clips.contains_key(name)
    }

    pub fn set_volumes(&self, v: Volumes) {
        self.mix.lock().unwrap().vol = v;
    }

    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed % 10_000) as f32 / 10_000.0
    }

    fn push(&mut self, name: &str, cat: Cat, gl: f32, gr: f32, rate: f64, looping: bool, fade_in: f32) -> Option<u64> {
        let clip = self.clips.get(name)?.clone();
        let now = macroquad::time::get_time();
        if !looping {
            if let Some(t) = self.last.get(name) {
                if now - t < 0.03 {
                    return None;
                }
            }
            self.last.insert(name.to_string(), now);
        }
        self.log.push((now, name.to_string()));
        let id = self.next;
        self.next += 1;
        let mut m = self.mix.lock().unwrap();
        if m.voices.len() >= MAX_VOICES {
            if let Some(i) = m.voices.iter().position(|v| !v.looping) {
                m.voices.remove(i);
            }
        }
        let step = if fade_in > 0.0 { 1.0 / (fade_in * RATE as f32) } else { 1.0 };
        m.voices.push(Voice { clip, pos: 0.0, rate, gl, gr, cat, looping, id, fade: if fade_in > 0.0 { 0.0 } else { 1.0 }, fade_to: 1.0, fade_step: step });
        Some(id)
    }

    /// Som sem posição (interface, tablet, objetos nas mãos).
    pub fn play(&mut self, name: &str, cat: Cat, vol: f32) {
        self.push(name, cat, vol, vol, 1.0, false, 0.0);
    }

    /// Como `play`, com pequena variação de altura (passos, teclas, cliques repetidos).
    pub fn play_var(&mut self, name: &str, cat: Cat, vol: f32, var: f32) {
        let r = 1.0 + (self.rand() * 2.0 - 1.0) * var;
        let g = vol * (1.0 - self.rand() * 0.15);
        self.push(name, cat, g, g, r as f64, false, 0.0);
    }

    /// Som posicionado no mundo: atenuação por distância e pan pelo lado do ouvinte.
    pub fn play_at(&mut self, name: &str, cat: Cat, vol: f32, src: Vec3, ear: Vec3, right: Vec3) {
        let (gl, gr) = spatial(src, ear, right);
        self.push(name, cat, gl * vol, gr * vol, 1.0, false, 0.0);
    }

    fn fade_out(&self, id: u64, secs: f32) {
        let mut m = self.mix.lock().unwrap();
        for v in m.voices.iter_mut().filter(|v| v.id == id) {
            v.fade_to = 0.0;
            v.fade_step = 1.0 / (secs.max(0.01) * RATE as f32);
        }
    }

    /// Troca o ambiente em loop com fusão (None = silêncio).
    pub fn ambient(&mut self, name: Option<&str>) {
        if self.ambient.as_ref().map(|a| a.0.as_str()) == name {
            return;
        }
        if let Some((_, id)) = self.ambient.take() {
            self.fade_out(id, 1.5);
        }
        if let Some(n) = name {
            if let Some(id) = self.push(n, Cat::Ambiente, 1.0, 1.0, 1.0, true, 1.5) {
                self.ambient = Some((n.to_string(), id));
            }
        }
    }

    /// Segunda camada de ambiente (ex.: ventoinha da estação quando sentado nela).
    pub fn layer(&mut self, name: Option<&str>) {
        if self.layer.as_ref().map(|a| a.0.as_str()) == name {
            return;
        }
        if let Some((_, id)) = self.layer.take() {
            self.fade_out(id, 0.8);
        }
        if let Some(n) = name {
            if let Some(id) = self.push(n, Cat::Ambiente, 0.8, 0.8, 1.0, true, 0.8) {
                self.layer = Some((n.to_string(), id));
            }
        }
    }

    pub fn active_voices(&self) -> usize {
        self.mix.lock().unwrap().voices.len()
    }

    // ------------------------------------------------------------ saídas

    fn start_recorder(&mut self, path: String) {
        let mix = self.mix.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let st = stop.clone();
        self.ok.store(true, Ordering::SeqCst);
        self.set_status(&format!("gravando em {}", path));
        let h = std::thread::spawn(move || {
            let t0 = std::time::Instant::now();
            let mut all: Vec<i16> = vec![];
            let mut buf = vec![0i16; CHUNK * 2];
            let mut frames: u64 = 0;
            while !st.load(Ordering::SeqCst) {
                let due = (t0.elapsed().as_secs_f64() * RATE as f64) as u64;
                while frames + CHUNK as u64 <= due {
                    mix.lock().unwrap().render(&mut buf, CHUNK);
                    all.extend_from_slice(&buf);
                    frames += CHUNK as u64;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            let _ = std::fs::write(&path, wav_bytes(&all, 2));
        });
        self.rec = Some((stop, h));
    }

    /// Modo por quadro: avança a mixagem `dt` segundos (chamado uma vez por quadro).
    pub fn tick(&mut self, dt: f64) {
        let Some((_, buf, frac)) = self.offline.as_mut() else { return };
        let want = dt * RATE as f64 + *frac;
        let n = want.floor() as usize;
        *frac = want - n as f64;
        let mut tmp = vec![0i16; n * 2];
        self.mix.lock().unwrap().render(&mut tmp, n);
        buf.extend_from_slice(&tmp);
    }

    /// Encerra a gravação (roteiros) e grava o arquivo.
    pub fn finish(&mut self) {
        if let Some((path, buf, _)) = self.offline.take() {
            let _ = std::fs::write(path, wav_bytes(&buf, 2));
        }
        if let Some((stop, h)) = self.rec.take() {
            stop.store(true, Ordering::SeqCst);
            let _ = h.join();
        }
    }

    #[cfg(target_os = "linux")]
    fn start_device(&mut self) {
        let mix = self.mix.clone();
        let status = self.status.clone();
        let ok = self.ok.clone();
        std::thread::spawn(move || match alsa::open() {
            Ok(dev) => {
                *status.lock().unwrap() = "ativo (ALSA, saída padrão do sistema)".into();
                ok.store(true, Ordering::SeqCst);
                let mut buf = vec![0i16; CHUNK * 2];
                loop {
                    mix.lock().unwrap().render(&mut buf, CHUNK);
                    if let Err(e) = dev.write(&buf) {
                        *status.lock().unwrap() = format!("interrompido: {}", e);
                        ok.store(false, Ordering::SeqCst);
                        return;
                    }
                }
            }
            Err(e) => {
                *status.lock().unwrap() = format!("indisponível — {}; o jogo segue em silêncio", e);
            }
        });
    }

    #[cfg(not(target_os = "linux"))]
    fn start_device(&mut self) {
        self.set_status("saída de som ainda não implementada nesta plataforma (o jogo segue em silêncio)");
    }
}

/// Ganho esquerdo/direito de uma fonte em `src` para um ouvinte em `ear` olhando com `right` à direita.
pub fn spatial(src: Vec3, ear: Vec3, right: Vec3) -> (f32, f32) {
    let d = src - ear;
    let dist = d.length();
    let att = 1.0 / (1.0 + 0.55 * (dist - 0.6).max(0.0));
    let p = if dist > 0.05 { (d / dist).dot(right).clamp(-1.0, 1.0) } else { 0.0 };
    let gl = 1.0 - p.max(0.0) * 0.7;
    let gr = 1.0 + p.min(0.0) * 0.7;
    (gl * att, gr * att)
}

#[cfg(target_os = "linux")]
mod alsa {
    //! ALSA por `dlopen`: o executável não depende de libasound para iniciar.
    use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void, CStr};

    type Open = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
    type SetParams = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
    type Nonblock = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
    type Writei = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulong) -> c_long;
    type Recover = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
    type Strerror = unsafe extern "C" fn(c_int) -> *const c_char;

    pub struct Dev {
        pcm: *mut c_void,
        writei: Writei,
        recover: Recover,
        strerror: Strerror,
    }
    unsafe impl Send for Dev {}

    unsafe fn sym<T>(lib: *mut c_void, name: &[u8]) -> Result<T, String> {
        let p = libc::dlsym(lib, name.as_ptr() as *const c_char);
        if p.is_null() {
            return Err(format!("símbolo ausente: {}", String::from_utf8_lossy(&name[..name.len() - 1])));
        }
        Ok(std::mem::transmute_copy(&p))
    }

    pub fn open() -> Result<Dev, String> {
        unsafe {
            let mut lib = libc::dlopen(b"libasound.so.2\0".as_ptr() as *const c_char, libc::RTLD_NOW);
            if lib.is_null() {
                lib = libc::dlopen(b"libasound.so\0".as_ptr() as *const c_char, libc::RTLD_NOW);
            }
            if lib.is_null() {
                return Err("biblioteca ALSA (libasound.so.2) não encontrada".into());
            }
            let open: Open = sym(lib, b"snd_pcm_open\0")?;
            let set_params: SetParams = sym(lib, b"snd_pcm_set_params\0")?;
            let nonblock: Nonblock = sym(lib, b"snd_pcm_nonblock\0")?;
            let writei: Writei = sym(lib, b"snd_pcm_writei\0")?;
            let recover: Recover = sym(lib, b"snd_pcm_recover\0")?;
            let strerror: Strerror = sym(lib, b"snd_strerror\0")?;
            let mut pcm: *mut c_void = std::ptr::null_mut();
            // abre sem bloquear (dispositivo ocupado não trava o jogo) e depois volta ao modo bloqueante
            let mut err = -1;
            for dev in [&b"default\0"[..], &b"pipewire\0"[..], &b"pulse\0"[..]] {
                err = open(&mut pcm, dev.as_ptr() as *const c_char, 0, 1);
                if err >= 0 {
                    break;
                }
            }
            if err < 0 {
                return Err(format!("nenhum dispositivo de som ({})", CStr::from_ptr(strerror(err)).to_string_lossy()));
            }
            nonblock(pcm, 0);
            // S16_LE = 2, RW_INTERLEAVED = 3, 2 canais, 44,1 kHz, reamostragem permitida, 80 ms de latência
            let e = set_params(pcm, 2, 3, 2, super::RATE, 1, 80_000);
            if e < 0 {
                return Err(format!("configuração recusada ({})", CStr::from_ptr(strerror(e)).to_string_lossy()));
            }
            Ok(Dev { pcm, writei, recover, strerror })
        }
    }

    impl Dev {
        pub fn write(&self, buf: &[i16]) -> Result<(), String> {
            let mut off = 0usize;
            let frames = buf.len() / 2;
            let mut fails = 0;
            while off < frames {
                let n = unsafe { (self.writei)(self.pcm, buf[off * 2..].as_ptr() as *const c_void, (frames - off) as c_ulong) };
                if n < 0 {
                    let r = unsafe { (self.recover)(self.pcm, n as c_int, 1) };
                    fails += 1;
                    if r < 0 || fails > 8 {
                        return Err(unsafe { CStr::from_ptr((self.strerror)(n as c_int)).to_string_lossy().into_owned() });
                    }
                    continue;
                }
                off += n as usize;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod t {
    use super::*;

    fn clip(v: Vec<f32>, ch: usize) -> Arc<Clip> {
        Arc::new(Clip { data: v, ch })
    }

    #[test]
    fn wav_ida_e_volta() {
        let s: Vec<i16> = (0..200).map(|i| ((i as f32 * 0.1).sin() * 12000.0) as i16).collect();
        let c = parse_wav(&wav_bytes(&s, 2)).unwrap();
        assert_eq!(c.ch, 2);
        assert_eq!(c.frames(), 100);
        assert!((c.data[3] - s[3] as f32 / 32768.0).abs() < 1e-6);
        assert!(parse_wav(b"lixo").is_err());
    }

    #[test]
    fn mixer_categorias_mudo_e_fim_da_voz() {
        let mut m = Mix::new();
        m.vol = Volumes { master: 1.0, cat: [0.0, 1.0, 1.0], muted: false };
        m.cur = [0.0, 1.0, 1.0];
        let c = clip(vec![0.5; 64], 1);
        m.voices.push(Voice { clip: c.clone(), pos: 0.0, rate: 1.0, gl: 1.0, gr: 1.0, cat: Cat::Efeitos, looping: false, id: 1, fade: 1.0, fade_to: 1.0, fade_step: 1.0 });
        m.voices.push(Voice { clip: c.clone(), pos: 0.0, rate: 1.0, gl: 1.0, gr: 1.0, cat: Cat::Ambiente, looping: true, id: 2, fade: 1.0, fade_to: 1.0, fade_step: 1.0 });
        let mut out = vec![0i16; 256];
        m.render(&mut out, 128);
        // só a voz de efeitos soa (ambiente em 0); ela acaba em 64 quadros
        assert!((out[0] as f32 / 32767.0 - 0.5).abs() < 0.01);
        assert_eq!(out[200], 0);
        assert_eq!(m.voices.len(), 1, "voz de efeito terminada é removida; o loop continua");
        m.vol.muted = true;
        m.cur = [0.0; 3];
        m.voices[0].cat = Cat::Efeitos;
        let mut out2 = vec![1i16; 256];
        m.render(&mut out2, 128);
        assert!(out2.iter().all(|&x| x == 0));
    }

    #[test]
    fn espacializacao() {
        let ear = Vec3::ZERO;
        let right = Vec3::X;
        let (l, r) = spatial(Vec3::new(2.0, 0.0, 0.0), ear, right);
        assert!(r > l * 2.0, "fonte à direita soa mais no canal direito");
        let (l2, r2) = spatial(Vec3::new(0.0, 0.0, -0.5), ear, right);
        assert!((l2 - r2).abs() < 1e-5 && l2 > 0.99, "à frente e perto: centro, sem atenuação");
        let (lf, _) = spatial(Vec3::new(0.0, 0.0, -6.0), ear, right);
        assert!(lf < 0.35, "longe atenua");
    }

    #[test]
    fn banco_de_sons_do_projeto_e_valido() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sounds");
        let mut n = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let c = parse_wav(&std::fs::read(e.path()).unwrap()).unwrap();
            assert!(c.frames() > 100, "{:?}", e.path());
            n += 1;
        }
        assert!(n >= 45);
    }
}
