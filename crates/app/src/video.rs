//! Vídeo de demonstração quadro a quadro (`PERICIA_VIDEO=<arquivo.mp4>`): envia os quadros
//! RGBA ao ffmpeg por um pipe. Usa `popen` da libc em vez de `std::process`, cuja
//! implementação atual exige símbolos da glibc 2.39 e impediria o executável de abrir
//! no Ubuntu 22.04 (glibc 2.35).

#[cfg(target_os = "linux")]
pub struct Pipe(*mut libc::FILE);

#[cfg(target_os = "linux")]
impl Pipe {
    pub fn open(path: &str, w: u32, h: u32) -> Option<Pipe> {
        let safe = path.replace('\'', "");
        let cmd = format!(
            "ffmpeg -loglevel error -y -f rawvideo -pix_fmt rgba -s {}x{} -r 30 -i - -vf vflip -c:v libx264 -preset veryfast -crf 22 -pix_fmt yuv420p '{}'\0",
            w, h, safe
        );
        let f = unsafe { libc::popen(cmd.as_ptr() as *const libc::c_char, b"w\0".as_ptr() as *const libc::c_char) };
        (!f.is_null()).then_some(Pipe(f))
    }
    pub fn frame(&mut self, rgba: &[u8]) {
        unsafe {
            libc::fwrite(rgba.as_ptr() as *const libc::c_void, 1, rgba.len(), self.0);
        }
    }
    pub fn close(self) {
        unsafe {
            libc::pclose(self.0);
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub struct Pipe;

#[cfg(not(target_os = "linux"))]
impl Pipe {
    pub fn open(_path: &str, _w: u32, _h: u32) -> Option<Pipe> {
        None
    }
    pub fn frame(&mut self, _rgba: &[u8]) {}
    pub fn close(self) {}
}

#[cfg(test)]
mod t {
    /// `std::process` puxa símbolos da glibc 2.39 (pidfd_spawnp) e o binário deixaria de abrir
    /// no Ubuntu 22.04. Nenhum arquivo do app pode usá-lo.
    #[test]
    fn app_nao_usa_std_process() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let txt = std::fs::read_to_string(e.path()).unwrap();
            let proib = ["std::process::Command", "process::Command::new", "Command::new("];
            for p in proib {
                let hits = txt.lines().filter(|l| l.contains(p) && !l.trim_start().starts_with("//") && !l.contains("let proib")).count();
                assert_eq!(hits, 0, "{:?} usa {}", e.path(), p);
            }
        }
    }
}
