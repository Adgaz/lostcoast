use glam::Vec3;

pub struct FlyCam {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub mouse_sensitivity: f32,
    forward: bool,
    back: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

impl FlyCam {
    pub fn new(pos: Vec3, target: Vec3) -> Self {
        let dir = (target - pos).normalize_or_zero();
        let yaw = dir.x.atan2(-dir.z);
        let pitch = dir.y.asin();
        Self {
            pos,
            yaw,
            pitch,
            speed: 4.0,
            mouse_sensitivity: 0.0025,
            forward: false,
            back: false,
            left: false,
            right: false,
            up: false,
            down: false,
        }
    }

    pub fn dir(&self) -> Vec3 {
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        Vec3::new(sy * cp, sp, -cy * cp).normalize()
    }

    pub fn target(&self) -> Vec3 {
        self.pos + self.dir()
    }

    pub fn update(&mut self, dt: f32) {
        let fwd = self.dir();
        let right = fwd.cross(Vec3::Y).normalize_or_zero();
        let up = Vec3::Y;
        let mut delta = Vec3::ZERO;
        if self.forward {
            delta += fwd;
        }
        if self.back {
            delta -= fwd;
        }
        if self.right {
            delta += right;
        }
        if self.left {
            delta -= right;
        }
        if self.up {
            delta += up;
        }
        if self.down {
            delta -= up;
        }
        if delta.length_squared() > 0.0 {
            self.pos += delta.normalize() * self.speed * dt;
        }
    }

    pub fn on_key(&mut self, key: &str, pressed: bool) {
        match key {
            "KeyW" => self.forward = pressed,
            "KeyS" => self.back = pressed,
            "KeyA" => self.left = pressed,
            "KeyD" => self.right = pressed,
            "Space" => self.up = pressed,
            "ShiftLeft" | "ShiftRight" => self.down = pressed,
            _ => {}
        }
    }

    pub fn on_mouse(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * self.mouse_sensitivity;
        self.pitch -= dy * self.mouse_sensitivity;
        let bound = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = self.pitch.clamp(-bound, bound);
    }
}
