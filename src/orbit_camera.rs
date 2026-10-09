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

/// Base de rayos calculada una sola vez por frame. Antes `ray()` reconstruía
/// la orientación completa de la cámara para cada píxel del framebuffer.
#[derive(Clone, Copy)]
pub struct CameraRayGrid {
    perspective: bool,
    eye: Vec3,
    forward: Vec3,
    first_sample: Vec3,
    step_x: Vec3,
    step_y: Vec3,
}

pub struct CameraRayRow {
    perspective: bool,
    eye: Vec3,
    sample: Vec3,
    step_x: Vec3,
    forward: Vec3,
}

impl CameraRayGrid {
    pub fn row(self, y: usize) -> CameraRayRow {
        CameraRayRow {
            perspective: self.perspective,
            eye: self.eye,
            sample: self.first_sample + self.step_y * y as f32,
            step_x: self.step_x,
            forward: self.forward,
        }
    }
}

impl CameraRayRow {
    pub fn next(&mut self) -> Ray {
        let ray = if self.perspective {
            Ray::new(self.eye, self.sample.normalize_or_zero())
        } else {
            Ray::new(self.sample, self.forward)
        };
        self.sample = self.sample + self.step_x;
        ray
    }
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

    #[cfg(test)]
    pub fn ray_direction(&self, x: usize, y: usize, width: usize, height: usize) -> Vec3 {
        let forward = (self.target - self.eye).normalize_or_zero();
        let right = forward.cross(self.up).normalize_or_zero();
        let camera_up = right.cross(forward);
        let aspect = width as f32 / height as f32;
        let scale = (self.fov_y * 0.5).tan();
        let screen_x = (2.0 * (x as f32 + 0.5) / width as f32 - 1.0) * aspect * scale;
        let screen_y = (1.0 - 2.0 * (y as f32 + 0.5) / height as f32) * scale;

        (forward + right * screen_x + camera_up * screen_y).normalize_or_zero()
    }

    #[cfg(test)]
    pub fn ray(&self, x: usize, y: usize, width: usize, height: usize) -> Ray {
        let mut row = self.ray_grid(width, height).row(y);
        for _ in 0..x {
            row.next();
        }
        row.next()
    }

    pub fn ray_grid(&self, width: usize, height: usize) -> CameraRayGrid {
        let forward = (self.target - self.eye).normalize_or_zero();
        let right = forward.cross(self.up).normalize_or_zero();
        let camera_up = right.cross(forward);
        let aspect = width as f32 / height as f32;
        let half_height = if self.projection == Projection::Perspective {
            (self.fov_y * 0.5).tan()
        } else {
            let offset = self.target - self.eye;
            offset.dot(offset).sqrt() * (self.fov_y * 0.5).tan()
        };
        let half_width = aspect * half_height;
        let step_x = right * (2.0 * half_width / width as f32);
        let step_y = camera_up * (-2.0 * half_height / height as f32);
        let top_left = if self.projection == Projection::Perspective {
            forward - right * half_width + camera_up * half_height
        } else {
            self.eye - right * half_width + camera_up * half_height
        };

        CameraRayGrid {
            perspective: self.projection == Projection::Perspective,
            eye: self.eye,
            forward,
            first_sample: top_left + (step_x + step_y) * 0.5,
            step_x,
            step_y,
        }
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
        self.eye = self.target + offset.normalize_or_zero() * new_distance;
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

    #[test]
    fn cached_ray_grid_matches_the_perspective_camera_math() {
        let camera = OrbitCamera::new(Vec3::new(8.0, 6.0, 10.0), Vec3::new(0.0, 0.5, -1.0), 0.75);
        let x = 137;
        let y = 42;
        let expected = camera.ray_direction(x, y, 320, 180);
        let mut row = camera.ray_grid(320, 180).row(y);
        let actual = (0..=x).map(|_| row.next()).last().unwrap();

        assert!((actual.direction.x - expected.x).abs() < 0.0001);
        assert!((actual.direction.y - expected.y).abs() < 0.0001);
        assert!((actual.direction.z - expected.z).abs() < 0.0001);
    }
}
