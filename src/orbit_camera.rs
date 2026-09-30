use crate::{math::Vec3, ray::Ray};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Projection {
    Perspective,
    Orthographic,
}

pub struct OrbitCamera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f32,
    projection: Projection,
}

impl OrbitCamera {
    pub fn new(eye: Vec3, target: Vec3, fov_y: f32) -> Self {
        Self {
            eye,
            target,
            up: Vec3::new(0.0, 1.0, 0.0),
            fov_y,
            projection: Projection::Perspective,
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

    pub fn ray(&self, x: usize, y: usize, width: usize, height: usize) -> Ray {
        let direction = self.ray_direction(x, y, width, height);
        if self.projection == Projection::Perspective {
            return Ray::new(self.eye, direction);
        }

        let forward = (self.target - self.eye).normalize();
        let right = forward.cross(&self.up).normalize();
        let camera_up = right.cross(&forward);
        let distance = (self.target - self.eye).dot(self.target - self.eye).sqrt();
        let aspect = width as f32 / height as f32;
        let half_height = distance * (self.fov_y * 0.5).tan();
        let screen_x = (2.0 * (x as f32 + 0.5) / width as f32 - 1.0) * aspect * half_height;
        let screen_y = (1.0 - 2.0 * (y as f32 + 0.5) / height as f32) * half_height;
        Ray::new(self.eye + right * screen_x + camera_up * screen_y, forward)
    }

    pub fn toggle_projection(&mut self) {
        self.projection = match self.projection {
            Projection::Perspective => Projection::Orthographic,
            Projection::Orthographic => Projection::Perspective,
        };
    }

    pub fn projection(&self) -> Projection {
        self.projection
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

    #[test]
    fn orthographic_rays_keep_their_direction_but_shift_their_origin() {
        let mut camera = OrbitCamera::new(Vec3::new(8.0, 6.0, 10.0), Vec3::default(), 0.75);
        camera.toggle_projection();

        let left = camera.ray(0, 50, 200, 100);
        let right = camera.ray(199, 50, 200, 100);
        assert_eq!(camera.projection(), Projection::Orthographic);
        assert!((left.direction.x - right.direction.x).abs() < 0.0001);
        assert!((left.direction.y - right.direction.y).abs() < 0.0001);
        assert!((left.direction.z - right.direction.z).abs() < 0.0001);
        assert!((left.origin.x - right.origin.x).abs() > 0.1);
    }
}
