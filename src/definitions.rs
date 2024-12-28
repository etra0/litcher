use std::marker::PhantomData;

use crate::pointer::*;
use imgui::Condition;
use lazy_re::lazy_re;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// For this game, we actually don't care which values are in here since we'll only copy/clone
/// those values from one struct to another, so this abstraction without any impl is more than
/// enough.
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct RotationMatrix([f32; 4 * 3]);

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct LightSettings {
    pub color: Color,
    pub radius: f32,
    pub brightness: f32,
    pub attenuation: f32,
}

/// CR4CameraDirector is the struct that contains the camera on all times, even on cinematics and
/// when going on a horse, so this is the struct we'll get our fancy camera position.
/// The way we get this camera is:
/// CR4Player -> World -> CR4CameraDirector.
#[lazy_re]
#[repr(C, packed)]
pub struct CR4CameraDirector {
    #[lazy_re(offset = 0x70)]
    pub pos: Position,

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

/// This struct will contain the light pointer that's created inside the game's memory alongside
/// with some external parameters we need for the UI/Control. We need to have an own copy of the
/// color for imgui to work properly.
/// Every LightContainer should have an unique id since imgui uses it as unique tokens.
pub struct LightContainer {
    pub light: LightType,

    // our settings
    pub attach_camera: bool,
    pub color: [f32; 4],
    pub open: bool,
    pub id: String,
}

impl LightContainer {
    pub fn new(light: LightType, id: usize) -> Self {
        Self {
            light,
            attach_camera: false,
            color: [1.; 4],
            open: true,
            id: format!("Light {}", id),
        }
    }

    /// LightContainer::update_render needs to be called for every render loop to update all
    /// parameters of the lights. We could technically optimize this to only be called when
    /// something on imgui changes but we don't care for that right now.
    pub fn update_render(&mut self, world: usize) {
        match &mut self.light {
            LightType::SpotLight(l) => {
                l.update_render(world);
            }
            LightType::PointLight(l) => {
                l.update_render(world);
            }
        }
    }

    /// Soft remove the light from the game.
    /// We trust the MemoryPool to actually clean this pointer, we just disable its visibility.
    pub fn remove_light(mut self, world: usize) {
        self.light.get_light_mut().is_enabled = false;
        self.update_render(world);
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

    pub fn render_window(&mut self, ui: &mut imgui::Ui) {
        if !self.open {
            return;
        }

        ui.window(&self.id)
            .size([350.0, 510.0], Condition::FirstUseEver)
            .opened(&mut self.open)
            .build(|| {
                let light = self.light.get_light_mut();
                // TODO: Revisit this!
                ui.color_picker4("color picker", &mut self.color);
                light.light_settings.color = self.color.into();
                let mut brightness = light.light_settings.brightness;
                let mut radius = light.light_settings.radius;
                let mut attenuation = light.light_settings.attenuation;
                let mut casting_mode = light.shadow_casting_mode as usize;
                let mut position: [f32; 3] = light.entity.pos.into();
                let mut shadow_blend_factor = light.shadow_blend_factor;

                imgui::Drag::new("Position")
                    .range(f32::MIN, f32::MAX)
                    .speed(0.1)
                    .build_array(ui, &mut position);

                imgui::Drag::new("Brightness")
                    .range(0.1, 100000.0)
                    .speed(if ui.is_key_index_down(VK_SHIFT as _) {
                        20.0
                    } else {
                        1.0
                    })
                    .build(ui, &mut brightness);

                ui.slider_config("Radius", f32::MIN, f32::MAX)
                    .range(0.1, 180.0)
                    .build(&mut radius);

                ui.slider_config("Shadow blend", f32::MIN, f32::MAX)
                    .range(0.0001, 1.0)
                    .build(&mut shadow_blend_factor);

                ui.slider_config("Attenuation", f32::MIN, f32::MAX)
                    .range(0.0001, 1.0)
                    .build(&mut attenuation);

                const SHADOWS_OPTIONS: [&'static str; 3] = [
                    "0 - No shadows",
                    "1 - Characters and objects",
                    "2 - Characters only",
                ];
                ui.combo("Shadow cast", &mut casting_mode, &[0, 1, 2], |&i| {
                    SHADOWS_OPTIONS[i].into()
                });

                ui.checkbox("Is enabled", &mut light.is_enabled);
                ui.checkbox("Attach to camera", &mut self.attach_camera);

                light.entity.pos = position.into();
                light.light_settings.brightness = brightness;
                light.light_settings.radius = radius;
                light.light_settings.attenuation = attenuation;
                light.shadow_casting_mode = casting_mode as _;
                light.shadow_blend_factor = shadow_blend_factor;

                ui.separator();

                match &mut self.light {
                    LightType::PointLight(pl) => pl.render_ui(ui),
                    LightType::SpotLight(spl) => spl.render_ui(ui),
                };
            });
    }
}

// Dummy for the Entity.
pub struct EmptyVT;

/// Parent entity. Since lights and players are objects, they share this struct, and in here we
/// also have the position and rotation of the object. Every object has its own virtual function
/// table so we need to be generic in that member of the struct.
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

/// This differs with the lightentity for the CR4Player, it's useful to have it different to the
/// lightentity mainly for the two pointers (ptr00 and ptr01) because from that we can extract the
/// CR4CameraDirector from the CR4Player.
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
pub struct LightEntityVT {
    // set_flags also triggers a re-render of the light.
    // 48 * 0x8
    #[lazy_re(offset = 384)]
    pub set_flags: unsafe extern "C" fn(light: &mut LightEntity, world: usize),
}

#[lazy_re]
#[repr(C, packed)]
pub struct LightEntity {
    pub entity: Entity<LightEntityVT>,

    #[lazy_re(offset = 0x130)]
    pub light_settings: LightSettings,

    #[lazy_re(offset = 0x164)]
    pub is_enabled: bool,

    #[lazy_re(offset = 0x16C)]
    pub shadow_blend_factor: f32,
    pub shadow_casting_mode: u32,
    pub shadow_fade_distance: f32,
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
        memory_pool: &'static mut MemoryPool<Self>,
        position: Position,
        rot: RotationMatrix,
        world: usize,
    ) -> &'static mut Self {
        let light_ptr = memory_pool.new_light();

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

    pub fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.text("Spotlight specific");
        let mut inner_angle = self.inner_angle;
        let mut outer_angle = self.outer_angle;
        let mut softness = self.softness;

        ui.slider_config("Inner angle", f32::MIN, f32::MAX)
            .range(0.1, outer_angle - 1.0)
            .build(&mut inner_angle);

        ui.slider_config("Outer angle", f32::MIN, f32::MAX)
            .range(0.1, 180.0)
            .build(&mut outer_angle);

        ui.slider_config("Softness", f32::MIN, f32::MAX)
            .range(0.1, 100.0)
            .build(&mut softness);

        self.outer_angle = outer_angle;
        self.inner_angle = inner_angle;
        if self.outer_angle < self.inner_angle {
            self.inner_angle = self.outer_angle - 1.;
        }
        self.softness = softness;
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
        memory_pool: &'static mut MemoryPool<Self>,
        position: Position,
        rot: RotationMatrix,
        world: usize,
    ) -> &'static mut Self {
        let light_ptr = memory_pool.new_light();

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

    pub fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.text("Pointlight specific");
        let mut cache_static_shadows: bool = self.cache_static_shadows != 0;
        let mut dynamic_shadow_face_mask: bool = self.dynamic_shadow_face_mask != 0;

        ui.checkbox("Cache static shadows", &mut cache_static_shadows);
        ui.checkbox("Dynamic Shadow Face Mask", &mut dynamic_shadow_face_mask);

        self.cache_static_shadows = cache_static_shadows as _;
        self.dynamic_shadow_face_mask = (dynamic_shadow_face_mask as u8) * 0x3F;
    }

    pub fn update_render(&mut self, world: usize) {
        unsafe { (self.light.entity.vt.set_flags)(&mut self.light, world) };
    }
}

impl LightEntity {
    /// This is a hacky way to check when the game 'deleted' some light, since it somewhat 'garbage
    /// collects' it, it also marks that specific field with the `0x22`, so we can check that flag
    /// every render loop to delete light references that are incorrect.
    pub fn should_get_deleted(&self) -> bool {
        (self.entity.flags & 0x22) != 0
    }
}

pub trait LightTypeTrait {}
impl LightTypeTrait for SpotLight {}
impl LightTypeTrait for PointLight {}

#[lazy_re]
#[repr(C, packed)]
struct MemoryPoolVT<T: LightTypeTrait + 'static> {
    // 25 * 0x8
    #[lazy_re(offset = 200)]
    spawn_object: unsafe extern "C" fn(*mut MemoryPool<T>) -> &'static mut T,
}

/// Most object in the game are created through a MemoryPool<T>, where T corresponds the actual
/// object to be created. There's a global function that uses the MemoryPool pointer that adds an
/// element of type T to the pool, so we need to keep track of two memory pools in this case:
/// SpotLight and PointLight.
/// Also, memory pools have a global pointer in the game where we get the pointer itself, so we can
/// be sure MemoryPools are unique per T.
#[lazy_re]
#[repr(C, packed)]
pub struct MemoryPool<T: LightTypeTrait + 'static> {
    vt: &'static MemoryPoolVT<T>,
    #[lazy_re(offset = 0x110)]
    clean_this: usize,
    _marker: PhantomData<T>,
}

impl<T: LightTypeTrait> MemoryPool<T> {
    pub fn new_light(&mut self) -> &'static mut T {
        self.clean_this = 0;
        let result = unsafe { (self.vt.spawn_object)(self as _) };
        result
    }
}

// Since these are game constants, we can make sure that at least those pointers will live as long
// as the game is running.
pub struct MainMemoryPools {
    pub spotlight: Pointer<MemoryPool<SpotLight>>,
    pub pointlight: Pointer<MemoryPool<PointLight>>,
}

/// CR4Player has the pointer to the World, a struct we need for MemoryPool to create an object and
/// to update the light's render since lights won't have any parent. From the world, we can also
/// extract the CR4CameraDirector, where we get the position and rotation of the current camera.
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

    pub fn get_camera(&self) -> Option<&'static CR4CameraDirector> {
        let layer = unsafe { self.0.read()?.ptr00? };
        let world = layer.ptr01?;

        let camera_dir: &'static CR4CameraDirector = unsafe { std::mem::transmute(world) };

        Some(camera_dir)
    }

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
