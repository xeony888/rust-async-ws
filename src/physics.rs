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
    pub fn update(&mut self, friction: f64, duration: f64) {
        let s = 1000_f64 / duration;
        self.x += self.vx * s;
        self.y += self.vy * s;
        self.vx = clamp_f64(self.vx * s * friction);
        self.vy = clamp_f64(self.vy * s * friction);
    }
    pub fn distance(&self, other: &Moveable2d) -> f64 {
        return f64::sqrt((self.x - other.x).powi(2) + (self.y - other.y).powi(2));
    }
    pub fn collide(&mut self, other: &mut Moveable2d) {
        let nx = other.x - self.x;
        let ny = other.y - self.y;
        let len = f64::sqrt(nx.powi(2) + ny.powi(2));
        let nx = nx / len;
        let ny = ny / len;

        let dvx = other.vx - self.vx;
        let dvy = other.vy - self.vy;
        let relative_velocity = dvx * nx + dvy * ny;

        if relative_velocity < 0.0 {
            let impulse = 2.0 * relative_velocity / (1.0 + 1.0); // Equal masses assumed

            self.vx += impulse * nx;
            self.vy += impulse * ny;
            other.vx -= impulse * nx;
            other.vy -= impulse * ny;
        }
    }
}
pub const PUCK_RADIUS: f64 = 4.0;
pub fn check_radial_collision(m1: &Moveable2d, m2: &Moveable2d, min_radius: f64) -> bool {
    let distance = m1.distance(&m2);
    return distance < min_radius;
}
