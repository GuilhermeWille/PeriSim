//! Controle em primeira pessoa: WASD/setas para andar (Shift corre), mouse para olhar
//! (botão direito pressionado, ou Tab para capturar o cursor), colisão circular contra
//! paredes e mobília, balanço sutil da cabeça ao caminhar.

use crate::world::{World, EYE};
use macroquad::prelude::*;

pub struct Player {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub captured: bool,
    bob_t: f32,
    pub case: String,
    pub fov: f32,
    /// Sensibilidade do mouse (multiplicador) e eixo vertical invertido — das configurações.
    pub sens: f32,
    pub invert_y: bool,
    /// Passada concluída neste quadro (Some(correndo)) — usada pelo som dos passos.
    pub step: Option<bool>,
}

const RADIUS: f32 = 0.28;

impl Player {
    pub fn new() -> Player {
        Player { pos: vec3(0.0, 0.0, 2.0), yaw: std::f32::consts::PI, pitch: -0.12, captured: false, bob_t: 0.0, case: String::new(), fov: 70f32.to_radians(), sens: 1.0, invert_y: false, step: None }
    }

    pub fn place(&mut self, pos: Vec3, yaw: f32) {
        self.pos = pos;
        self.yaw = yaw;
        self.pitch = -0.15;
    }

    pub fn dir(&self) -> Vec3 {
        vec3(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), self.yaw.cos() * self.pitch.cos())
    }

    pub fn eye(&self) -> Vec3 {
        self.pos + vec3(0.0, EYE + (self.bob_t * 9.0).sin() * 0.012, 0.0)
    }

    pub fn camera(&self, viewport: Rect) -> Camera3D {
        let e = self.eye();
        Camera3D { position: e, target: e + self.dir(), up: vec3(0.0, 1.0, 0.0), fovy: self.fov, aspect: Some(viewport.w / viewport.h), z_near: 0.03, z_far: 900.0, ..Default::default() }
    }

    pub fn set_captured(&mut self, c: bool) {
        self.captured = c;
        set_cursor_grab(c);
        show_mouse(!c);
    }

    /// Atualiza; retorna true se a câmera se moveu (para o modo de economia de energia).
    pub fn update(&mut self, world: &World, keys: bool, look_area: bool) -> bool {
        let dt = get_frame_time().min(0.05);
        let mut moved = false;
        let d = mouse_delta_position();
        if self.captured || (look_area && is_mouse_button_down(MouseButton::Right)) {
            if d.length_squared() > 0.0 {
                self.yaw += d.x * 2.4 * self.sens;
                let dy = if self.invert_y { -d.y } else { d.y };
                self.pitch = (self.pitch + dy * 1.8 * self.sens).clamp(-1.35, 1.35);
                moved = true;
            }
        }
        if !keys {
            return moved;
        }
        if is_key_down(KeyCode::Left) {
            self.yaw += 1.8 * dt;
            moved = true;
        }
        if is_key_down(KeyCode::Right) {
            self.yaw -= 1.8 * dt;
            moved = true;
        }
        if is_key_down(KeyCode::PageUp) {
            self.pitch = (self.pitch + 1.2 * dt).min(1.35);
            moved = true;
        }
        if is_key_down(KeyCode::PageDown) {
            self.pitch = (self.pitch - 1.2 * dt).max(-1.35);
            moved = true;
        }
        let fwd = vec2(self.yaw.sin(), self.yaw.cos());
        let right = vec2(-fwd.y, fwd.x);
        let mut wish = Vec2::ZERO;
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            wish += fwd;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            wish -= fwd;
        }
        if is_key_down(KeyCode::A) {
            wish -= right;
        }
        if is_key_down(KeyCode::D) {
            wish += right;
        }
        if wish.length_squared() > 0.0 {
            let speed = if is_key_down(KeyCode::LeftShift) { 2.6 } else { 1.5 };
            let step = wish.normalize() * speed * dt;
            self.try_move(world, step);
            let before = (self.bob_t * 9.0 / std::f32::consts::TAU).floor();
            self.bob_t += dt * if speed > 2.0 { 1.4 } else { 1.0 };
            // um passo por ciclo do balanço da cabeça (no ponto mais baixo)
            if (self.bob_t * 9.0 / std::f32::consts::TAU).floor() != before {
                self.step = Some(speed > 2.0);
            }
            moved = true;
        }
        moved
    }

    fn blocked(world: &World, p: Vec2) -> bool {
        let (mn, mx) = world.bounds;
        if p.x < mn.x + RADIUS || p.x > mx.x - RADIUS || p.y < mn.y + RADIUS || p.y > mx.y - RADIUS {
            return true;
        }
        world.colliders.iter().any(|(a, b)| {
            let q = vec2(p.x.clamp(a.x, b.x), p.y.clamp(a.y, b.y));
            (q - p).length_squared() < RADIUS * RADIUS
        })
    }

    /// Move com deslizamento ao longo das superfícies (resolve eixo a eixo).
    pub fn try_move(&mut self, world: &World, step: Vec2) {
        let p = vec2(self.pos.x, self.pos.z);
        let nx = vec2(p.x + step.x, p.y);
        let p1 = if !Self::blocked(world, nx) { nx } else { p };
        let nz = vec2(p1.x, p1.y + step.y);
        let p2 = if !Self::blocked(world, nz) { nz } else { p1 };
        self.pos.x = p2.x;
        self.pos.z = p2.y;
    }
}
