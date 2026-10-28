//! Gerador pseudoaleatório determinístico (mulberry32). Toda aleatoriedade do
//! simulador passa por aqui para garantir que semente + versão reproduzam o caso.

#[derive(Clone, Debug)]
pub struct Rng {
    s: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng { s: seed }
    }
    pub fn for_label(seed: u32, label: &str) -> Self {
        Rng::new(seed ^ fnv1a(label))
    }
    pub fn next_u32(&mut self) -> u32 {
        self.s = self.s.wrapping_add(0x6d2b79f5);
        let mut t = self.s;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        t ^ (t >> 14)
    }
    pub fn f64(&mut self) -> f64 {
        self.next_u32() as f64 / 4294967296.0
    }
    /// Inteiro uniforme em [min, max] (inclusivo).
    pub fn int(&mut self, min: i64, max: i64) -> i64 {
        if max <= min {
            return min;
        }
        min + (self.f64() * ((max - min + 1) as f64)) as i64
    }
    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        assert!(!v.is_empty(), "pick em lista vazia");
        let i = (self.f64() * v.len() as f64) as usize;
        &v[i.min(v.len() - 1)]
    }
    pub fn shuffle<T: Clone>(&mut self, v: &[T]) -> Vec<T> {
        let mut a = v.to_vec();
        for i in (1..a.len()).rev() {
            let j = (self.f64() * (i + 1) as f64) as usize;
            a.swap(i, j.min(i));
        }
        a
    }
    pub fn sample<T: Clone>(&mut self, v: &[T], n: usize) -> Vec<T> {
        let mut s = self.shuffle(v);
        s.truncate(n);
        s
    }
    pub fn digits(&mut self, n: usize) -> String {
        (0..n).map(|_| char::from(b'0' + self.int(0, 9) as u8)).collect()
    }
    pub fn hex(&mut self, n: usize) -> String {
        (0..n).map(|_| b"0123456789ABCDEF"[self.int(0, 15) as usize] as char).collect()
    }
    pub fn fork(&self, label: &str) -> Rng {
        Rng::new(self.s ^ fnv1a(label))
    }
}

/// FNV-1a 32 bits — apenas para derivar sementes (não é usado para integridade).
pub fn fnv1a(s: &str) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}
