use serde::{Deserialize, Serialize};

use crate::math::clamp_f64;

#[derive(Serialize, Deserialize)]
pub struct Moveable2d {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
}
impl Moveable2d {
    pub fn new() -> Self {
        return Moveable2d {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
        };
    }
    pub fn update(&mut self, friction: f64) {
        self.x += self.vx;
        self.y += self.vy;
        self.vx = clamp_f64(self.vx * friction);
        self.vy = clamp_f64(self.vy * friction);
    }
    pub fn distance(&self, other: Moveable2d) -> f64 {
        return f64::sqrt((self.x - other.x).powi(2) + (self.y - other.y).powi(2));
    }
    pub fn collide(&mut self, other: &mut Moveable2d) {}
}
pub fn check_radial_collision(m1: Moveable2d, m2: Moveable2d, min_radius: f64) -> bool {
    let distance = m1.distance(m2);
    return distance < min_radius;
}
