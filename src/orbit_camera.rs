use crate::math::Vec3;

pub struct OrbitCamera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f32,
}

impl OrbitCamera {
    pub fn new(eye: Vec3, target: Vec3, fov_y: f32) -> Self {
        Self {
            eye,
            target,
            up: Vec3::new(0.0, 1.0, 0.0),
            fov_y,
        }
    }

    pub fn ray_direction(&self, x: usize, y: usize, width: usize, height: usize) -> Vec3 {
        let forward = (self.target - self.eye).normalize();
        let right = forward.cross(&self.up).normalize();
        let camera_up = right.cross(&forward);
        let aspect = width as f32 / height as f32;
        let scale = (self.fov_y * 0.5).tan();
        let screen_x = (2.0 * (x as f32 + 0.5) / width as f32 - 1.0) * aspect * scale;
        let screen_y = (1.0 - 2.0 * (y as f32 + 0.5) / height as f32) * scale;

        (forward + right * screen_x + camera_up * screen_y).normalize()
    }
}
