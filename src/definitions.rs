use std::mem::MaybeUninit;

use crate::pointer::*;
use imgui::{Condition, Window};
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
pub struct RotationMatrix([f32; 4 * 3]);

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct LightSettings {
    pub color: Color,
    pub radius: f32,
    pub brightness: f32,
}

unsafe impl Sync for LightEntity {}
unsafe impl Send for LightEntity {}

unsafe impl Send for LightContainer {}
unsafe impl Sync for LightContainer {}

#[lazy_re]
#[repr(C, packed)]
pub struct CR4CameraDirector {
    #[lazy_re(offset = 0x70)]
    pub pos: Position,

    pub unk00: f32,

    #[lazy_re(offset = 0x1A0)]
    pub rot_matrix: RotationMatrix,
}

pub enum LightType {
    SpotLight(&'static mut SpotLight),
    PointLight(&'static mut PointLight),
}

impl LightType {
    pub fn get_light_mut(&mut self) -> &mut LightEntity {
        match self {
            Self::SpotLight(SpotLight { light, .. }) => light,
            Self::PointLight(PointLight { light, .. }) => light,
        }
    }
}

// This is the struct that *we* control
pub struct LightContainer {
    pub light: LightType,

    // our settings
    pub attach_camera: bool,
    pub color: [f32; 4],
}

impl LightContainer {
    pub fn new(light: LightType) -> Self {
        Self {
            light,
            attach_camera: false,
            color: [255.; 4],
        }
    }

    pub fn set_pos_rot(&mut self, pos: Position, rot: RotationMatrix) {
        match &mut self.light {
            LightType::PointLight(pl) => {
                pl.set_pos_rot(pos, rot);
            }
            LightType::SpotLight(spl) => {
                spl.set_pos_rot(pos, rot);
            }
        };
    }

    pub fn render_window(&mut self, ui: &mut imgui::Ui, ix: usize) {
        Window::new(format!("Light {}", ix))
            .size([300.0, 210.0], Condition::FirstUseEver)
            .build(ui, || {
                let mut light = self.light.get_light_mut();

                imgui::ColorPicker::new("color picker", &mut self.color).build(ui);
                light.light_settings.color = self.color.into();

                let mut brightness = light.light_settings.brightness;
                let mut radius = light.light_settings.radius;
                let mut casting_mode = light.shadow_casting_mode as usize;
                let mut position: [f32; 3] = light.entity.pos.into();
                imgui::InputFloat3::new(ui, "Position", &mut position)
                    .no_horizontal_scroll(false)
                    .build();

                imgui::Slider::new("Brightness", 0.1, 100000.0).build(ui, &mut brightness);

                imgui::Slider::new("Radius", f32::MIN, f32::MAX)
                    .range(0.1, 180.0)
                    .build(ui, &mut radius);

                const shadows: [&str; 3] = [
                    "0 - No shadows",
                    "1 - Characters and objects",
                    "2 - Characters only",
                ];
                ui.combo("Shadow casting mode", &mut casting_mode, &[0, 1, 2], |&i| {
                    shadows[i].into()
                });

                ui.checkbox("Is enabled", &mut light.is_enabled);
                ui.checkbox("Attach to camera", &mut self.attach_camera);

                light.entity.pos = position.into();
                light.light_settings.brightness = brightness;
                light.light_settings.radius = radius;
                light.shadow_casting_mode = casting_mode as _;

                ui.separator();

                match &mut self.light {
                    LightType::PointLight(pl) => {
                        ui.text("Pointlight specific");
                        let mut cache_static_shadows: bool = pl.cache_static_shadows != 0;
                        let mut dynamic_shadow_face_mask: bool = pl.dynamic_shadow_face_mask != 0;

                        ui.checkbox("Cache static shadows", &mut cache_static_shadows);
                        ui.checkbox("Dynamic Shadow Face Mask", &mut dynamic_shadow_face_mask);

                        pl.cache_static_shadows = cache_static_shadows as _;
                        pl.dynamic_shadow_face_mask = (dynamic_shadow_face_mask as u8) * 0x3F;
                    }
                    LightType::SpotLight(spl) => {
                        ui.text("Spotlight specific");
                        let mut inner_angle = spl.inner_angle;
                        let mut outer_angle = spl.outer_angle;
                        let mut softness = spl.softness;

                        imgui::Slider::new("Inner angle", f32::MIN, f32::MAX)
                            .range(0.1, outer_angle - 1.0)
                            .build(ui, &mut inner_angle);

                        imgui::Slider::new("Outer angle", f32::MIN, f32::MAX)
                            .range(0.1, 180.0)
                            .build(ui, &mut outer_angle);

                        imgui::Slider::new("Softness", f32::MIN, f32::MAX)
                            .range(0.1, 100.0)
                            .build(ui, &mut softness);

                        spl.outer_angle = outer_angle;
                        spl.inner_angle = inner_angle;
                        if spl.outer_angle < spl.inner_angle {
                            spl.inner_angle = spl.outer_angle - 1.;
                        }
                        spl.softness = softness;
                    }
                };
            });
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

    #[lazy_re(offset = 0x70)]
    pub rot_matrix: RotationMatrix,

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

    #[lazy_re(offset = 0x70)]
    pub rot_matrix: RotationMatrix,

    #[lazy_re(offset = 0xA0)]
    pub pos: Position,
}

#[lazy_re]
#[repr(C, packed)]
pub struct LightEntity {
    pub entity: Entity<LightEntityVT>,

    #[lazy_re(offset = 0x130)]
    pub light_settings: LightSettings,

    #[lazy_re(offset = 0x164)]
    pub is_enabled: bool,

    #[lazy_re(offset = 0x170)]
    pub shadow_casting_mode: u32,
    pub shadow_fade_distance: u32,
    pub shadow_fade_range: f32,
}

#[lazy_re]
#[repr(C, packed)]
pub struct SpotLight {
    pub light: LightEntity,

    #[lazy_re(offset = 0x180)]
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub softness: f32,
}

impl SpotLight {
    pub fn new(
        memory_pool: &'static mut MemoryPool,
        memory_pool_func: MemoryPoolFunc,
        position: Position,
        rot: RotationMatrix,
        world: usize,
    ) -> &'static mut Self {
        let light_ptr: &'static mut Self =
            unsafe { std::mem::transmute((memory_pool_func)(memory_pool, 0, 1, 0)) };

        light_ptr.light.entity.pos = position;
        light_ptr.light.entity.rot_matrix = rot;

        light_ptr.light.light_settings.brightness = 1000.0;
        light_ptr.light.light_settings.radius = 5.0;
        light_ptr.inner_angle = 30.0;
        light_ptr.outer_angle = 45.0;
        light_ptr.softness = 2.;

        light_ptr.light.shadow_casting_mode = 1;
        light_ptr.light.is_enabled = true;

        unsafe { (light_ptr.light.entity.vt.set_flags)(&mut light_ptr.light, world) };

        println!("New SpotLight: {:p}", light_ptr);
        light_ptr
    }

    pub fn set_pos_rot(&mut self, pos: Position, rot: RotationMatrix) {
        self.light.entity.pos = pos;
        self.light.entity.rot_matrix = rot;
    }

    pub fn update_render(&mut self, world: usize) {
        unsafe { (self.light.entity.vt.set_flags)(&mut self.light, world) };
    }
}

#[lazy_re]
#[repr(C, packed)]
pub struct PointLight {
    pub light: LightEntity,

    #[lazy_re(offset = 0x180)]
    pub cache_static_shadows: u8,
    pub dynamic_shadow_face_mask: u8,
}

impl PointLight {
    pub fn new(
        memory_pool: &'static mut MemoryPool,
        memory_pool_func: MemoryPoolFunc,
        position: Position,
        rot: RotationMatrix,
        world: usize,
    ) -> &'static mut Self {
        let light_ptr: &'static mut Self =
            unsafe { std::mem::transmute((memory_pool_func)(memory_pool, 0, 1, 0)) };

        light_ptr.light.entity.pos = position;
        light_ptr.light.entity.rot_matrix = rot;

        light_ptr.light.light_settings.brightness = 1000.0;
        light_ptr.light.light_settings.radius = 5.0;

        light_ptr.cache_static_shadows = 1;
        light_ptr.dynamic_shadow_face_mask = 1;

        light_ptr.light.shadow_casting_mode = 1;
        light_ptr.light.is_enabled = true;

        unsafe { (light_ptr.light.entity.vt.set_flags)(&mut light_ptr.light, world) };

        println!("New PointLight: {:p}", light_ptr);
        light_ptr
    }

    pub fn set_pos_rot(&mut self, pos: Position, rot: RotationMatrix) {
        self.light.entity.pos = pos;
        self.light.entity.rot_matrix = rot;
    }

    pub fn update_render(&mut self, world: usize) {
        unsafe { (self.light.entity.vt.set_flags)(&mut self.light, world) };
    }
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

    pub fn get_world(&self) -> Option<usize> {
        let layer = unsafe { self.0.read()?.ptr00? };
        let world = layer.ptr00? as *const _ as usize;

        Some(world)
    }

    pub fn get_camera(&self) -> Option<&'static ScriptedEntity<EmptyVT>> {
        let camera = unsafe { self.0.read()?.ptr01? };

        Some(camera)
    }

    // TODO: Remove this!
    pub fn get_camera2(&self) -> Option<&'static CR4CameraDirector> {
        let layer = unsafe { self.0.read()?.ptr00? };
        let world = layer.ptr01?;

        let camera_dir: &'static CR4CameraDirector = unsafe { std::mem::transmute(world) };

        Some(camera_dir)
    }

    // TODO: Check if this actually copies internal values.
    pub fn should_update(&self) -> bool {
        *self.0.should_update.lock().unwrap() || self.0.last_value.lock().unwrap().is_none()
    }

    pub fn updated(&mut self) {
        *self.0.should_update.lock().unwrap() = false;
        *self.0.last_value.lock().unwrap() = None;
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
