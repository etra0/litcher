//! Some things that are useful for debugging:
//! The render_proxy function is at [$process + 02a0f50]. It receives two arguments, the light
//! pointer and the world. It's useful to hook into it to steal the world pointer.
//! Offset of the CR4Player Memory Pool: [$process + 2d56848]
#![feature(once_cell)]
#![allow(unused_imports)]

use memory_rs::internal::{
    injections::{Detour, Inject, Injection},
    memory::resolve_module_path,
    process_info::ProcessInfo,
};
use std::{ffi::c_void, sync::Arc};
use windows_sys::Win32::{
    System::{
        Console::{AllocConsole, FreeConsole},
        LibraryLoader::{FreeLibraryAndExitThread, GetModuleHandleA, GetProcAddress},
    },
    UI::Input::{
        KeyboardAndMouse::*,
        XboxController::{XInputGetState, XINPUT_STATE},
    },
};

use lazy_re::{lazy_re, LazyRe};

use log::*;
use simplelog::*;

use hudhook::hooks::dx11::ImguiDX11Hooks;
use hudhook::hooks::{ImguiRenderLoop, ImguiRenderLoopFlags};
use imgui_dx11::imgui::{Condition, Window};

mod definitions;
mod pointer;

use definitions::*;
use pointer::*;

struct Context {
    memory_pools: MainMemoryPools,
    lights: Vec<LightKindContainer>,
    show: bool,
    player: CR4Player,
    memory_pool_func: MemoryPoolFunc,
}


unsafe fn deref_pointer(addr: usize, offsets: &[usize]) -> Option<usize> {
    let mut current_addr = std::ptr::read((addr) as *const usize);
    for offset in offsets {
        if current_addr == 0 {
            return None;
        }

        current_addr = std::ptr::read((current_addr + offset) as *const usize);
    }

    Some(current_addr)
}

impl Context {
    fn new() -> Self {
        println!("Initializing");
        hudhook::utils::alloc_console();
        let proc_info = ProcessInfo::new(None).unwrap();

        let memory_pools = MainMemoryPools {
            pointlight: Pointer::new(proc_info.region.start_address + 0x2c46878, Vec::new()),
            spotlight: Pointer::new(proc_info.region.start_address + 0x2c46900, Vec::new()),
        };

        // base pointer:
        // [$process + 2a51558]
        // [0x230, 0x88, 0x30, 0x30]
        //          ^ CR4Player
        //
        // Another option:
        // [$process + 0x2c5dd38]
        // [0x1A8, 0x40] <= CR4Player
        let player: Pointer<ScriptedEntity<EmptyVT>> = Pointer::new(proc_info.region.start_address + 0x2c5dd38, vec![0x1A8, 0x40]);

        let lights = Vec::new();

        let memory_pool_func: MemoryPoolFunc = unsafe { std::mem::transmute(proc_info.region.start_address + 0x03c400) };

        Self {
            memory_pools,
            lights,
            show: false,
            player: CR4Player::new(player),
            memory_pool_func,
        }
    }

    // TODO: fix the position stuff.
    unsafe fn spawn_new_light(&mut self, kind: LightKind) {
        // TODO: Ugh, find a way to just do one match.
        let light = match kind {
            LightKind::SpotLight => {
                (self.memory_pool_func)(self.memory_pools.spotlight.read().unwrap(), 0, 1, 0)
            }
            LightKind::PointLight => {
                (self.memory_pool_func)(self.memory_pools.pointlight.read().unwrap(), 0, 1, 0)
            }
        };

        let camera = self.player.get_camera().unwrap();
        light.entity.pos = camera.pos;
        light.entity.rotations = camera.rotations;

        light.light.brightness = 1000.0;
        light.light.radius = 180.0;
        light.inner_angle = 0.1;
        light.outer_angle = 180.0;
        light.softness = 1.;

        (light.entity.vt.set_flags)(light, self.player.get_world().unwrap());
        light.shadow_casting_mode = 2;
        light.is_enabled = true;

        println!("new light: {:p}", light);
        unsafe {
            (light.entity.vt.set_flags)(light, self.player.get_world().unwrap());
        }

        match kind {
            LightKind::SpotLight => self.lights.push(LightKindContainer::SpotLight(light)),
            LightKind::PointLight => self.lights.push(LightKindContainer::PointLight(light)),
        };
    }
}

// TODO: Fix this, only temporary solution.
impl Default for Position {
    fn default() -> Self {
        Self {
            x: -403.67,
            y: -253.33,
            z: 8.0,
        }
    }
}

fn render_window_per_light(ui: &mut imgui_dx11::imgui::Ui, light: &mut LightEntity, ix: usize) {
    Window::new(format!("Light {}", ix))
        .size([300.0, 210.0], Condition::FirstUseEver)
        .build(ui, || {
            let mut colors: [f32; 4] = light.light.color.into();
            let cp = imgui::ColorPicker::new("color picker", &mut colors);
            if cp.build(&ui) {
                light.light.color = colors.into();
            }

            let mut brightness = light.light.brightness;
            let mut radius = light.light.radius;

            let mut inner_angle = light.inner_angle;
            let mut outer_angle = light.outer_angle;
            let mut softness = light.softness;
            imgui::Slider::new("Brightness", f32::MIN, f32::MAX)
                .range(0.1, 10000.0)
                .build(&ui, &mut brightness);
            imgui::Slider::new("radius", f32::MIN, f32::MAX)
                .range(0.1, 360.0)
                .build(&ui, &mut radius);

            imgui::Slider::new("Inner angle", f32::MIN, f32::MAX)
                .range(0.1, 360.0)
                .build(&ui, &mut inner_angle);

            imgui::Slider::new("Outer angle", f32::MIN, f32::MAX)
                .range(0.1, 360.0)
                .build(&ui, &mut outer_angle);

            imgui::Slider::new("Softness", f32::MIN, f32::MAX)
                .range(0.1, 10000.0)
                .build(&ui, &mut softness);

            let mut position: [f32; 3] = light.entity.pos.into();
            imgui::InputFloat3::new(&ui, "Position", &mut position)
                .no_horizontal_scroll(false)
                .build();

            ui.checkbox("is enabled", &mut light.is_enabled);

            light.entity.pos = position.into();
            light.light.brightness = brightness;
            light.light.radius = radius;
            light.inner_angle = inner_angle;
            light.outer_angle = outer_angle;
            light.softness = softness;
        });
}

impl ImguiRenderLoop for Context {
    fn render(&mut self, ui: &mut imgui_dx11::imgui::Ui, flags: &ImguiRenderLoopFlags) {
        // Force a read every render to avoid crashes.
        let _world = self.player.get_world();
        if _world.is_none() {
            self.lights.clear();
            self.player.updated();

            println!("World is changed or none");
        }

        // TODO: Revisit this logic, it might be not needed anymore.
        if self.player.should_update() {
            println!("Player was updated");
            self.lights.clear();
            self.player.updated();
        }


        if flags.focused
            && !ui.io().want_capture_keyboard
            && ui.is_key_index_pressed_no_repeat(VK_F4 as _)
        {
            self.show = !self.show;
        }

        // TODO: Remove this
        if ui.is_key_index_pressed_no_repeat(VK_F5 as _) {
            // Before we do the clear, let's just deactivate all the current lights.
            for light in self.lights.iter_mut() {
                match light {
                    LightKindContainer::SpotLight(l) | LightKindContainer::PointLight(l) => {
                        l.is_enabled = false;
                        unsafe { (l.entity.vt.set_flags)(l, self.player.get_world().unwrap()) };
                    },
                }
            }
            self.lights.clear();
            self.player.updated();
        }

        if self.show {
            ui.set_mouse_cursor(Some(imgui::MouseCursor::Arrow));
            Window::new("Main window")
                .size([200.0, 200.0], Condition::FirstUseEver)
                .build(ui, || {
                    if ui.button("Spawn new pointlight") {
                        unsafe {
                            self.spawn_new_light(LightKind::PointLight);
                        }
                    }

                    if ui.button("Spawn new spotlight") {
                        unsafe {
                            self.spawn_new_light(LightKind::SpotLight);
                        }
                    }
                });


            // ptr00 is invalid when the light is no longer being used (i.e. it's GC'ed)
            self.lights.retain(|x: &definitions::LightKindContainer| {
                let ptr = x.downcast();
                !ptr.should_get_deleted()
            });

            for (i, light) in self.lights.iter_mut().enumerate() {
                let light = light.downcast_mut();
                render_window_per_light(ui, light, i);
                unsafe {
                    (light.entity.vt.set_flags)(light, self.player.get_world().unwrap());
                }
            }
        } else {
            ui.set_mouse_cursor(None);
        }
    }

    fn should_block_messages(&self, io: &imgui::Io) -> bool {
        _ = io;
        false
    }
}

hudhook::hudhook!(Context::new().into_hook::<ImguiDX11Hooks>());