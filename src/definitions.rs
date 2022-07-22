use crate::pointer::*;
use lazy_re::{lazy_re, LazyRe};

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Rotations {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub _unused: f32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Light {
    pub color: Color,
    pub radius: f32,
    pub brightness: f32,
}

unsafe impl Sync for LightEntity {}
unsafe impl Send for LightEntity {}

unsafe impl Send for LightWrapper {}
unsafe impl Sync for LightWrapper {}

#[lazy_re]
#[repr(C, packed)]
pub struct CR4CameraDirector {
    #[lazy_re(offset = 0x70)]
    pub pos: Position,

    pub unk00: f32,

    // Guessing that this is the Z rotation, I don't actually know, neither do I care.
    pub z_rot: f32,
    pub y_rot: f32,
    pub x_rot: f32,
}

impl CR4CameraDirector {
    // Ugh, i guess that by design we have to pass this function around. It kinda sucks but it is
    // what we have right now.
    pub fn get_rot(&self, calc_const: &impl Fn(f32) -> f32) -> Rotations {
        let y = calc_const(self.x_rot) * calc_const(self.y_rot);
        Rotations {
            x: (self.x_rot.to_radians() - std::f32::consts::PI).sin(),
            z: (-self.y_rot.to_radians() + std::f32::consts::PI).sin(),
            y, _unused: 0.0
        }
    }
}

pub enum LightKind {
    SpotLight,
    PointLight,
}

pub struct LightWrapper {
    pub light: &'static mut LightEntity,

    // our settings
    pub attach_camera: bool,
    pub kind: LightKind,
}

impl LightWrapper {
    pub fn new(kind: LightKind, light: &'static mut LightEntity) -> Self {
        Self {
            light,
            attach_camera: false,
            kind
        }
    }
}

#[lazy_re]
#[repr(C, packed)]
pub struct LightEntityVT {
    // 44 * 0x8
    #[lazy_re(offset = 352)]
    pub get_parent: unsafe extern "C" fn(light: &LightEntity) -> &'static CLayer,
    // set_flags also triggers a re-render of the light.
    // 48 * 0x8
    #[lazy_re(offset = 384)]
    pub set_flags: unsafe extern "C" fn(light: &mut LightEntity, world: usize),
}

#[lazy_re]
#[repr(C, packed)]
pub struct CLayer {
    #[lazy_re(offset = 0x68)]
    pub world: usize,
}

// Dummy for the Entity.
pub struct EmptyVT;

#[lazy_re]
#[repr(C, packed)]
pub struct Entity<VT: 'static> {
    pub vt: &'static VT,

    #[lazy_re(offset = 0x30)]
    pub parent: usize,

    #[lazy_re(offset = 0x54)]
    pub flags: u32,

    #[lazy_re(offset = 0x80)]
    pub rotations: Rotations,

    #[lazy_re(offset = 0xA0)]
    pub pos: Position,
}

#[lazy_re]
#[repr(C, packed)]
pub struct ScriptedEntity<VT: 'static> {
    pub vt: &'static VT,

    // In the C4RPlayer this is the DynamicLayer
    #[lazy_re(offset = 0x30)]
    pub ptr00: Option<&'static ScriptedEntity<EmptyVT>>,
    // CCustomCamera
    pub ptr01: Option<&'static ScriptedEntity<EmptyVT>>,

    #[lazy_re(offset = 0x80)]
    pub rotations: Rotations,

    #[lazy_re(offset = 0xA0)]
    pub pos: Position,
}

#[lazy_re]
#[repr(C, packed)]
pub struct LightEntity {
    pub entity: Entity<LightEntityVT>,

    #[lazy_re(offset = 0x130)]
    pub light: Light,

    #[lazy_re(offset = 0x164)]
    pub is_enabled: bool,

    #[lazy_re(offset = 0x170)]
    pub shadow_casting_mode: u32,
    pub shadow_fade_distance: u32,
    pub shadow_fade_range: f32,

    // Stuff specific to spotlights
    #[lazy_re(offset = 0x180)]
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub softness: f32,
}

impl LightEntity {
    pub fn should_get_deleted(&self) -> bool {
        return (self.entity.flags & 0x22) != 0;
    }
}

unsafe impl Sync for MemoryPool {}
unsafe impl Send for MemoryPool {}

#[lazy_re]
#[repr(C, packed)]
pub struct MemoryPool {
    pub vt: *const MemoryPoolVT,

    #[lazy_re(offset = 0x110)]
    pub light: &'static LightEntity,
}

#[lazy_re]
#[repr(C, packed)]
pub struct MemoryPoolVT {
    #[lazy_re(offset = 200)]
    pub spawn_entity: unsafe extern "C" fn(memory_pool: &MemoryPool) -> &'static mut LightEntity,
}

pub type MemoryPoolFunc = unsafe extern "C" fn(
    memory_pool: &MemoryPool,
    unused: usize,
    marker: u8,
    light: usize,
) -> &'static mut LightEntity;

// Since these are game constants, we can make sure that at least those pointers will live as long
// as the game is running.
pub struct MainMemoryPools {
    pub spotlight: Pointer<MemoryPool>,
    pub pointlight: Pointer<MemoryPool>,
}

// TODO: rethink if this is the best way to abstract this, since it could lead to confusion.
pub struct CR4Player(Pointer<ScriptedEntity<EmptyVT>>);

impl CR4Player {
    pub fn new(player: Pointer<ScriptedEntity<EmptyVT>>) -> Self {
        Self(player)
    }

    pub fn get_world(&mut self) -> Option<usize> {
        let layer = unsafe { self.0.read()?.ptr00? };
        let world = layer.ptr00? as *const _ as usize;

        Some(world)
    }

    pub fn get_camera(&mut self) -> Option<&'static ScriptedEntity<EmptyVT>> {
        let camera = unsafe { self.0.read()?.ptr01? };

        Some(camera)
    }

    // TODO: Remove this!
    pub fn get_camera2(&mut self) -> Option<&'static CR4CameraDirector> {
        let layer = unsafe { self.0.read()?.ptr00? };
        let world = layer.ptr01?;

        let camera_dir: &'static CR4CameraDirector = unsafe { std::mem::transmute(world) };

        Some(camera_dir)
    }

    pub fn should_update(&self) -> bool {
        self.0.should_update || self.0.last_value.is_none()
    }

    pub fn updated(&mut self) {
        self.0.should_update = false;
        self.0.last_value = None;
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl From<[f32; 4]> for Color {
    fn from(col: [f32; 4]) -> Self {
        Self {
            red: (col[0] * 255.0) as u8,
            green: (col[1] * 255.0) as u8,
            blue: (col[2] * 255.0) as u8,
            alpha: (col[3] * 255.0) as u8,
        }
    }
}

impl From<Color> for [f32; 4] {
    fn from(col: Color) -> Self {
        [
            (col.red as f32) / 255.0,
            (col.green as f32) / 255.0,
            (col.blue as f32) / 255.0,
            (col.alpha as f32) / 255.0,
        ]
    }
}

impl From<[f32; 3]> for Position {
    fn from(arr: [f32; 3]) -> Self {
        Position {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }
}

impl From<Position> for [f32; 3] {
    fn from(pos: Position) -> Self {
        [pos.x, pos.y, pos.z]
    }
}
