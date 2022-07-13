use lazy_re::lazy_re;

#[repr(C, packed)]
#[derive(Copy, Clone)]
#[lazy_re]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
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

#[repr(C, packed)]
#[lazy_re]
pub struct Light {
    pub color: Color,
    pub radius: f32,
    pub brightness: f32,
}

#[repr(C, packed)]
#[lazy_re]
pub struct LightEntity {
    vt: usize,

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
}

pub struct LightFunctions {
    pub ctor_caller: unsafe extern "C" fn(
        memory_pool: usize,
        _unused: usize,
        marker: u8,
        light: *mut LightEntity,
    ) -> *mut LightEntity,
    pub flag_setter: unsafe extern "C" fn(light: *mut LightEntity, world: usize),
    pub render_update: unsafe extern "C" fn(light: *mut LightEntity, world: usize),
}

impl LightFunctions {
    pub fn new(base_addr: usize) -> Self {
        Self {
            render_update: unsafe { std::mem::transmute(base_addr + 0x2a0f50) },
            ctor_caller: unsafe { std::mem::transmute(base_addr + 0x03c400) },
            flag_setter: unsafe { std::mem::transmute(base_addr + 0x02a06f0) },
        }
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
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
