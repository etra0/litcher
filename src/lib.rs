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

use hudhook::hooks::{ImguiRenderLoop, ImguiRenderLoopFlags};
use hudhook::hooks::dx11::ImguiDX11Hooks;
use imgui_dx11::imgui::{Condition, Window};

mod definitions;
use definitions::*;

struct Context {
    memory_pools: MainMemoryPools,
    world: usize,
    lights: Vec<LightKindContainer>,
    show: bool,
}


impl Context {
    fn new() -> Self {
        println!("Initializing");
        hudhook::utils::alloc_console();
        let proc_info = ProcessInfo::new(None).unwrap();

        let memory_pools = MainMemoryPools {
            pointlight: unsafe { std::mem::transmute(*((proc_info.region.start_address + 0x2c46878) as *const usize)) },
            spotlight: unsafe { std::mem::transmute(*((proc_info.region.start_address  + 0x2c46900) as *const usize)) },
        };

        // TODO: Check how to do this!!
        let world = 0x000001D8815C78A0;
        let lights = Vec::new();

        Self {
            memory_pools,
            world,
            lights,
            show: false,
        }
    }

    // TODO: fix the position stuff.
    unsafe fn spawn_new_light(&mut self, kind: LightKind) {
        // TODO: Ugh, find a way to just do one match.
        let light = match kind {
            LightKind::SpotLight => 
                ((*self.memory_pools.spotlight.vt).spawn_entity)(self.memory_pools.spotlight),
            LightKind::PointLight => 
                ((*self.memory_pools.pointlight.vt).spawn_entity)(self.memory_pools.pointlight),
        };

        light.pos = Position::default();
        light.light.brightness = 1000.0;
        light.light.radius = 180.0;
        light.inner_angle = 0.1;
        light.outer_angle = 180.0;
        light.softness = 1.;

        (light.vt.set_flags)(light, self.world);
        light.shadow_casting_mode = 2;


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

            let mut position: [f32; 3] = light.pos.into();
            imgui::InputFloat3::new(&ui, "Position", &mut position)
                .no_horizontal_scroll(false)
                .build();

            ui.checkbox("is enabled", &mut light.is_enabled);

            light.pos = position.into();
            light.light.brightness = brightness;
            light.light.radius = radius;
            light.inner_angle = inner_angle;
            light.outer_angle = outer_angle;
            light.softness = softness;
        });

}

impl ImguiRenderLoop for Context {
    fn render(&mut self, ui: &mut imgui_dx11::imgui::Ui, flags: &ImguiRenderLoopFlags) {

        if flags.focused && !ui.io().want_capture_keyboard && ui.is_key_index_pressed_no_repeat(VK_F4 as _) {
            self.show = !self.show;
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

            for (i, light) in self.lights.iter_mut().enumerate() {
                let light = match light {
                    LightKindContainer::SpotLight(l) => l,
                    LightKindContainer::PointLight(l) => l
                    };
                render_window_per_light(ui, light, i);
                unsafe {
                    (light.vt.set_flags)(light, self.world);
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
