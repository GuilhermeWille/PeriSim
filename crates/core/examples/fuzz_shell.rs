use pericia_core::casegen::generate;
use pericia_core::engine::Sim;
use pericia_core::model::*;
use pericia_core::os::{shell::run_line, Os, Shell};
fn main() {
    let lines = ["|", "||", "&&", ";", ">", "<", "echo \"abc", "echo 'x", "cat", "cat /dev/sda | head", "ls /*/*/*", "find / -name '*' | wc -l",
        "cd ..; cd ..; cd ..; pwd", "echo $UNDEF$$", "ls > /", "echo a > /dev/null", "rm -rf /", "sudo rm -rf /mnt", "mount", "umount /", "sudo mount -o rw /dev/sdb1 /mnt/x",
        "sudo mkdir -p /mnt/a && sudo mount -o ro,loop ~/casos/*/M-01.dd /mnt/a && ls /mnt/a", "grep -r a / | head -3", "cat /proc/*", "ls -la ~ ~/casos", "echo ${", "a=1 b=2; echo $a$b",
        "history | tail", "man", "man zzz", "stat", "file", "sha256sum", "sha256sum /dev/sdb", "tree / | head", "head -n -5 x", "tail -n abc /etc/passwd", "sort < /etc/hostname",
        "cp / /tmp", "mv ~ ~/x", "touch /dev/x", "mkdir /", "df -h", "lsblk -f", "dmesg | tail", "pericia", "pericia imagem", "pericia imagem m-01; pericia imagem m-01", "ls '*'", "ls \\*",
        "echo `ls`", "echo $(ls)", "cat ~/casos/*/M-01.dd | head -c 10", "sudo", "sudo sudo ls", "exit", "clear"];
    let mut runs = 0; let mut pan = 0;
    for m in ModuleId::ALL { for d in 1..=4u8 { for seed in [1u32, 2, 3] {
        let r = std::panic::catch_unwind(|| {
            let mut sim = Sim::new(generate(m, d, seed));
            sim.exec("bloqueio m-01");
            let mut os = Os::new(&sim);
            let mut sh = Shell::new(&os.home);
            for l in lines { os.sync(&sim); let _ = run_line(&mut os, &mut sim, &mut sh, l); }
            sim.audit.verify().is_ok()
        });
        runs += 1;
        match r { Err(_) => { pan += 1; eprintln!("PÂNICO {:?} D{} s{}", m, d, seed) } Ok(false) => { pan += 1; eprintln!("AUDIT {:?}", m) } _ => {} }
    }}}
    println!("execuções={} problemas={}", runs, pan);
}
