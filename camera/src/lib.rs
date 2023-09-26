use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};

#[derive(Debug, PartialEq, Clone)]
pub struct Control {
    pub position: glam::DVec3,
    pub rotation: glam::Vec2,
    pub focal_length: f32,
}

impl Control {
    pub fn rot(&self) -> glam::Mat3 {
        glam::Mat3::from_euler(
            glam::EulerRot::ZXY,
            self.rotation.x * 3.14,
            0.0,
            self.rotation.y / 2.0 * 3.14,
        )
    }
    pub fn look_dir(&self) -> glam::Vec3 {
        self.rot() * glam::Vec3::X
    }
    pub fn zero() -> Self {
        Self {
            position: glam::DVec3::ZERO,
            rotation: glam::Vec2::ZERO,
            focal_length: 1.0,
        }
    }
    pub fn clamp(&mut self) {
        self.rotation.y = self.rotation.y.clamp(-1.0, 1.0);
    }
    pub fn apply(&mut self, other: &Self) {
        self.position += other.position;
        self.rotation += other.rotation;
    }
    pub fn camera_matrix(&self) -> glam::Mat3 {
        let rot_mat = self.rot();
        let pointing = rot_mat * glam::Vec3::X;
        let up = rot_mat * glam::Vec3::Z;

        compute_camera(pointing, up)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogicalCamera {
    pub camera_rot: Mat3,
    pub camera_rot_invert: Mat3,
    //3 * 4 * 2 = 24
    pub camera_origin_int: glam::IVec3,
    pub camera_origin_sub: glam::Vec3,

    pub focal_length: f32,
}

impl From<Control> for LogicalCamera {
    fn from(c: Control) -> Self {
        (&c).into()
    }
}

impl From<&Control> for LogicalCamera {
    fn from(c: &Control) -> Self {
        let int = c.position.floor().as_ivec3();
        let f = (c.position - int.as_dvec3()).as_vec3();

        let mat = c.camera_matrix();
        Self {
            camera_rot: mat,
            camera_rot_invert: mat.inverse(),
            camera_origin_int: int,
            camera_origin_sub: f,
            focal_length: 1.0,
        }
    }
}

fn compute_camera(pointing: glam::Vec3, up: glam::Vec3) -> glam::Mat3 {
    let cross = pointing.cross(up);
    glam::Mat3 {
        x_axis: cross.normalize(),
        y_axis: up.normalize(),
        z_axis: pointing.normalize(),
    }
}
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct GpuCamera {
    // 48 + 48 = 96
    pub camera_rot: GPUMat3,
    pub camera_rot_invert: GPUMat3,
    //3 * 4 * 2 = 24
    pub camera_origin_int: glam::IVec3,
    pub focal_length: f32,
    pub camera_origin_sub: glam::Vec3,
    pub _pad: u32,
    // // 4
    // 4 * 4 * 4 = 16 * 4 = 64
}

impl From<&LogicalCamera> for GpuCamera {
    fn from(cam: &LogicalCamera) -> Self {
        Self {
            camera_rot: cam.camera_rot.into(),
            camera_rot_invert: cam.camera_rot_invert.into(),
            camera_origin_int: cam.camera_origin_int,
            camera_origin_sub: cam.camera_origin_sub,
            focal_length: cam.focal_length,
            _pad: 0,
        }
    }
}

#[derive(Copy, Clone, Pod, Zeroable, Debug)]
#[repr(C, align(16))]
pub struct GPUMat3 {
    // 3 * 4 + 4 = 16
    pub x_axis: glam::Vec3,
    pub _pad0: u32,
    // 3 * 4 + 4 = 16
    pub y_axis: glam::Vec3,
    pub _pad1: u32,
    // 3 * 4 + 4 = 16
    pub z_axis: glam::Vec3,
    pub _pad2: u32,
}

impl From<glam::Mat3> for GPUMat3 {
    fn from(m: glam::Mat3) -> Self {
        Self {
            x_axis: m.x_axis,
            y_axis: m.y_axis,
            z_axis: m.z_axis,
            _pad0: 0xFF,
            _pad1: 0xFF,
            _pad2: 0xFF,
        }
    }
}

#[derive(Copy, Clone, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct GpuCameraNormal {
    // 48 + 48 = 96
    pub camera_rot: GPUMat3,
    pub camera_rot_invert: GPUMat3,
    pub camera_origin: glam::Vec3,
    pub focal_length: f32,
    pub camera_4: Mat4,
    pub camera_proj: Mat4,
    pub camera_4_invert: Mat4,
}

impl From<&Control> for GpuCameraNormal {
    fn from(control: &Control) -> Self {
        let flip = Mat3::from_cols(-Vec3::X, Vec3::Z, -Vec3::Y);
        let mat = flip * control.camera_matrix();
        let inv = mat.inverse();
        let camera_4 = Mat4::from_mat3(inv) * Mat4::from_translation(-control.position.as_vec3());
        Self {
            camera_rot: mat.into(),
            camera_rot_invert: inv.into(),
            camera_origin: flip * control.position.as_vec3(),
            focal_length: control.focal_length,
            camera_4,
            camera_proj: Mat4::perspective_rh(3.14 / 4.0, 1.0, 0.2, 100.0),
            camera_4_invert: camera_4.inverse(),
        }
    }
}
