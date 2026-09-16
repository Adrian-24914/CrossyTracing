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

    pub fn orbit_y(&mut self, angle: f32) {
        let offset = self.eye - self.target;
        let cosine = angle.cos();
        let sine = angle.sin();
        self.eye = self.target
            + Vec3::new(
                offset.x * cosine - offset.z * sine,
                offset.y,
                offset.x * sine + offset.z * cosine,
            );
    }

    pub fn zoom(&mut self, amount: f32) {
        let offset = self.eye - self.target;
        let distance = offset.dot(offset).sqrt();
        let new_distance = (distance + amount).clamp(5.0, 30.0);
        self.eye = self.target + offset.normalize() * new_distance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_preserves_distance_and_zoom_changes_it() {
        let mut camera =
            OrbitCamera::new(Vec3::new(8.0, 6.0, 10.0), Vec3::new(0.0, 0.0, 0.0), 0.75);
        let initial_offset = camera.eye - camera.target;
        let initial_distance = initial_offset.dot(initial_offset).sqrt();

        camera.orbit_y(0.4);
        let orbit_offset = camera.eye - camera.target;
        let orbit_distance = orbit_offset.dot(orbit_offset).sqrt();
        assert!((initial_distance - orbit_distance).abs() < 0.0001);

        camera.zoom(-1.0);
        let zoom_offset = camera.eye - camera.target;
        let zoom_distance = zoom_offset.dot(zoom_offset).sqrt();
        assert!(zoom_distance < orbit_distance);
    }
}
