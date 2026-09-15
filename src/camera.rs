use nalgebra_glm::{dot, Vec3};

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f32,
}

#[derive(Clone, Copy)]
pub struct ProjectedPoint {
    pub x: f32,
    pub y: f32,
    pub inverse_depth: f32,
}

impl Camera {
    pub fn new(eye: Vec3, target: Vec3, fov_y: f32) -> Self {
        Self {
            eye,
            target,
            up: Vec3::new(0.0, 1.0, 0.0),
            fov_y,
        }
    }

    pub fn project(&self, point: &Vec3, width: usize, height: usize) -> Option<ProjectedPoint> {
        let forward = (self.target - self.eye).normalize();
        let right = forward.cross(&self.up).normalize();
        let camera_up = right.cross(&forward);
        let relative = point - self.eye;
        let depth = dot(&relative, &forward);
        if depth <= 0.1 {
            return None;
        }

        let aspect = width as f32 / height as f32;
        let focal = 1.0 / (self.fov_y * 0.5).tan();
        let ndc_x = dot(&relative, &right) * focal / (depth * aspect);
        let ndc_y = dot(&relative, &camera_up) * focal / depth;

        Some(ProjectedPoint {
            x: (ndc_x + 1.0) * 0.5 * width as f32,
            y: (1.0 - ndc_y) * 0.5 * height as f32,
            inverse_depth: 1.0 / depth,
        })
    }
}
