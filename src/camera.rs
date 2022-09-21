use crate::AppState;
use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(AppState::Main).with_system(Self::camera_move_system),
        )
        .add_system_set(SystemSet::on_update(AppState::Main).with_system(Self::camera_look_system))
        .add_system_set(SystemSet::on_update(AppState::Main).with_system(Self::camera_zoom_system));
    }
}

impl CameraPlugin {
    pub fn camera_move_system(
        mut camera: ResMut<CustomCamera>,
        input: Res<Input<KeyCode>>,
        time: Res<Time>,
    ) {
        let camera_speed: f32 = 2.5;
        let mut translation = Vec3::ZERO;
        let camera_right = camera.right();

        if input.pressed(KeyCode::W) {
            translation += camera.get_direction() * camera_speed;
        }
        if input.pressed(KeyCode::S) {
            translation -= camera.get_direction() * camera_speed;
        }
        if input.pressed(KeyCode::A) {
            translation -= camera_right * camera_speed;
        }
        if input.pressed(KeyCode::D) {
            translation += camera_right * camera_speed;
        }

        if translation != Vec3::ZERO {
            camera.translate(translation * time.delta_seconds());
        }
    }

    pub fn camera_look_system(
        mut camera: ResMut<CustomCamera>,
        mut mouse_motion: EventReader<MouseMotion>,
        time: Res<Time>,
    ) {
        let look_sensitivity: f32 = 0.1;
        let mut rotation_offset = Vec2::ZERO;

        for event in mouse_motion.iter() {
            rotation_offset += event.delta * look_sensitivity * time.delta_seconds();
        }

        if rotation_offset != Vec2::ZERO {
            camera.rotate(rotation_offset.x, -rotation_offset.y);
        }
    }

    fn camera_zoom_system(
        mut camera: ResMut<CustomCamera>,
        mut mouse_wheel: EventReader<MouseWheel>,
    ) {
        let sensitivity: f32 = 1.0;

        for event in mouse_wheel.iter() {
            camera.zoom(event.y * sensitivity);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CustomCamera {
    pub position: Vec3,
    // The positive z axis is going through the screen toward you
    // To move the camera backwards, move along the z-axis
    pub yaw: f32,
    pub pitch: f32,
    pub up: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl CustomCamera {
    pub fn get_view(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.get_direction(), self.up)
    }

    pub fn get_proj(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov.to_radians(),
            self.aspect_ratio,
            self.near,
            self.far,
        )
    }

    pub fn get_direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch += pitch;
    }

    pub fn translate(&mut self, position: Vec3) {
        self.position += position;
        // Uncomment this to keep the "player" on the ground - FPS camera
        // self.position.y = 0.0;
    }

    pub fn right(&self) -> Vec3 {
        self.get_direction().cross(self.up).normalize()
    }

    pub fn zoom(&mut self, amount: f32) {
        self.fov -= amount;
        self.fov = self.fov.clamp(1.0, 45.0);
    }
}
