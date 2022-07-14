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
    // set_flags also triggers a re-render of the light.
    // 48 * 0x8
    #[lazy_re(offset = 384)]
    pub set_flags: unsafe extern "C" fn(light: &mut LightEntity, world: usize),
}

#[lazy_re]
#[repr(C, packed)]
pub struct LightEntity {
    pub vt: &'static LightEntityVT,

    #[lazy_re(offset = 0xA0)]
    pub pos: Position,

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

pub struct MemoryPool {
    pub vt: *const MemoryPoolVT,
}

#[lazy_re]
#[repr(C, packed)]
pub struct MemoryPoolVT {
    #[lazy_re(offset = 200)]
    pub spawn_entity: unsafe extern "C" fn(memory_pool: &MemoryPool) -> &'static mut LightEntity,
}

// Since these are game constants, we can make sure that at least those pointers will live as long
// as the game is running.
pub struct MainMemoryPools {
    pub spotlight: &'static mut MemoryPool,
    pub pointlight: &'static mut MemoryPool,
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

