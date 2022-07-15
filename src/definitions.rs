use lazy_re::{lazy_re, LazyRe};
use crate::pointer::*;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Light {
    pub color: Color,
    pub radius: f32,
    pub brightness: f32,
}

// Since we manage our own memory, it's just easier to upgrade to static lifetimes.
pub enum LightKindContainer {
    SpotLight(&'static mut LightEntity),
    PointLight(&'static mut LightEntity),
}

pub enum LightKind {
    SpotLight,
    PointLight,
}

unsafe impl Sync for LightEntity {}
unsafe impl Send for LightEntity {}

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
    pub world: usize
}

// Dummy for the Entity.
pub struct EmptyVT;

#[repr(C, packed)]
pub struct Entity<VT: 'static> {
    pub vt: &'static VT,

    __pad00: [u8; 0x30 - std::mem::size_of::<usize>()],
    pub parent: usize,

    __pad01: [u8; 0xA0 - (0x30 + std::mem::size_of::<usize>())],
    pub pos: Position,
}

#[repr(C, packed)]
pub struct ScriptedEntity<VT: 'static> {
    pub vt: &'static VT,

    __pad00: [u8; 0x30 - std::mem::size_of::<usize>()],
    // In the C4RPlayer this is the DynamicLayer
    pub ptr00: &'static ScriptedEntity<VT>,
    // CCustomCamera
    pub ptr01: &'static ScriptedEntity<VT>,

    __pad01: [u8; 0xA0 - (0x30 + std::mem::size_of::<usize>()*2)],
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

pub type MemoryPoolFunc = unsafe extern "C" fn(memory_pool: &MemoryPool, unused: usize, marker: u8, light: usize) -> &'static mut LightEntity;


// Since these are game constants, we can make sure that at least those pointers will live as long
// as the game is running.
pub struct MainMemoryPools {
    pub spotlight: &'static mut MemoryPool,
    pub pointlight: &'static mut MemoryPool,
}

// TODO: rethink if this is the best way to abstract this, since it could lead to confusion.
pub struct CR4Player(Pointer<ScriptedEntity<EmptyVT>>);

impl CR4Player {

    pub fn new(player: Pointer<ScriptedEntity<EmptyVT>>) -> Self {
        Self(player)
    }

    pub fn get_world(&mut self) -> usize {
        let layer = unsafe { self.0.read().unwrap().ptr00 };
        let world = layer.ptr00 as *const _ as usize;

        world
    }

    pub fn get_camera(&mut self) -> &'static ScriptedEntity<EmptyVT> {
        let camera = unsafe { self.0.read().unwrap().ptr01 };

        camera
    }

    pub fn should_update(&self) -> bool {
        self.0.should_update
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

