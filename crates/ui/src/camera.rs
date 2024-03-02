/// The projection matrix used in the shaders.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    projection: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new_ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        CameraUniform {
            projection: [
                [2.0 / (right - left), 0.0, 0.0, 0.0],
                [0.0, 2.0 / (top - bottom), 0.0, 0.0],
                [0.0, 0.0, 1.0 / (near - far), 0.0],
                [
                    (right + left) / (left - right),
                    (top + bottom) / (bottom - top),
                    near / (near - far),
                    1.0,
                ],
            ],
        }
    }
}
