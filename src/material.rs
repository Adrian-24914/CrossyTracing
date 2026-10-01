use crate::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finish {
    Matte,
    Unlit,
    Glossy,
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub albedo: Color,
    pub finish: Finish,
    pub specular_strength: f32,
    pub shininess: f32,
    pub transparency: f32,
    /// Índice óptico: 1.0 no desvía el rayo; el agua usa aproximadamente 1.333.
    pub refraction_index: f32,
    pub reflectivity: f32,
}

impl Material {
    pub const fn matte(albedo: Color) -> Self {
        Self {
            albedo,
            finish: Finish::Matte,
            specular_strength: 0.0,
            shininess: 1.0,
            transparency: 0.0,
            refraction_index: 1.0,
            reflectivity: 0.0,
        }
    }

    pub const fn unlit(albedo: Color) -> Self {
        Self {
            albedo,
            finish: Finish::Unlit,
            specular_strength: 0.0,
            shininess: 1.0,
            transparency: 0.0,
            refraction_index: 1.0,
            reflectivity: 0.0,
        }
    }

    pub const fn glossy(albedo: Color, specular_strength: f32, shininess: f32) -> Self {
        Self {
            albedo,
            finish: Finish::Glossy,
            specular_strength,
            shininess,
            transparency: 0.0,
            refraction_index: 1.0,
            reflectivity: 0.0,
        }
    }

    pub const fn translucent_matte(albedo: Color, transparency: f32) -> Self {
        Self {
            albedo,
            finish: Finish::Matte,
            specular_strength: 0.0,
            shininess: 1.0,
            transparency,
            refraction_index: 1.0,
            reflectivity: 0.0,
        }
    }

    pub const fn reflective_glossy(
        albedo: Color,
        specular_strength: f32,
        shininess: f32,
        reflectivity: f32,
    ) -> Self {
        Self {
            albedo,
            finish: Finish::Glossy,
            specular_strength,
            shininess,
            transparency: 0.0,
            refraction_index: 1.0,
            reflectivity,
        }
    }

    pub const fn refractive(albedo: Color, transparency: f32, refraction_index: f32) -> Self {
        Self {
            albedo,
            finish: Finish::Matte,
            specular_strength: 0.0,
            shininess: 1.0,
            transparency,
            refraction_index,
            reflectivity: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_presets_keep_their_intended_finish() {
        let white = Material::unlit(Color::new(255, 255, 255));
        let water = Material::refractive(Color::new(52, 129, 164), 0.42, 1.333);
        let metal = Material::reflective_glossy(Color::new(174, 53, 45), 0.3, 28.0, 0.24);
        let ground = Material::matte(Color::new(78, 145, 82));

        assert_eq!(white.finish, Finish::Unlit);
        assert_eq!(white.albedo.to_hex(), 0xFFFFFF);
        assert_eq!(water.finish, Finish::Matte);
        assert!((water.transparency - 0.42).abs() < 0.0001);
        assert!((water.refraction_index - 1.333).abs() < 0.0001);
        assert!((metal.reflectivity - 0.24).abs() < 0.0001);
        assert_eq!(ground.finish, Finish::Matte);
    }
}
