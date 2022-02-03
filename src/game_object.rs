use std::rc::Rc;

use crate::vulkan::Model;

pub struct TransformComponent {
    pub translation: glam::Vec3,
    pub scale: glam::Vec3,
    pub rotation: glam::Vec3,
}

impl TransformComponent {
    pub fn mat4(&self) -> glam::Mat4 {
        let quat = glam::quat(self.rotation.x, self.rotation.y, self.rotation.z, 0.0);
        glam::Mat4::from_scale_rotation_translation(self.scale, quat, self.translation)
    }

    pub fn normal_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale(1.0 / self.scale)
    }
}

pub struct PointLightComponent {
    pub light_intensity: f32,
}

static mut CURRENT_ID: u8 = 0;

pub struct GameObject {
    pub id: u8,
    pub model: Option<Rc<Model>>,
    pub color: glam::Vec3,
    pub transform: TransformComponent,
    pub point_light: Option<PointLightComponent>,
}

impl GameObject {
    pub fn new(
        model: Option<Rc<Model>>,
        color: Option<glam::Vec3>,
        transform: Option<TransformComponent>,
    ) -> Self {
        let color = match color {
            Some(c) => c,
            None => glam::vec3(0.0, 0.0, 0.0),
        };

        let transform = match transform {
            Some(t) => t,
            None => TransformComponent {
                translation: glam::vec3(0.0, 0.0, 0.0),
                scale: glam::vec3(1.0, 1.0, 1.0),
                rotation: glam::vec3(0.0, 0.0, 0.0),
            }
        };

        let id = unsafe {
            CURRENT_ID
        };

        unsafe {
            CURRENT_ID += 1;
        }

        Self {
            id,
            model,
            color,
            transform,
            point_light: None,
        }
    }

    pub fn make_point_light(intensity: f32, radius: f32, color: glam::Vec3) -> Self {
        let mut game_object = Self::new(
            None,
            Some(color),
            Some(TransformComponent {
                translation: glam::vec3(0.0, 0.0, 0.0),
                scale: glam::vec3(radius, 0.0, 0.0),
                rotation: glam::vec3(0.0, 0.0, 0.0),
            }));

        game_object.point_light = Some(PointLightComponent {
            light_intensity: intensity,
        });

        game_object
    }
}
