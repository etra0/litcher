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

use hudhook::hooks::dx11::{ImguiRenderLoop, ImguiRenderLoopFlags};
use imgui_dx11::imgui::{Condition, Window};

mod util;
mod definitions;

use definitions::*;

struct Context {
    memory_pool: usize,
    world: usize,
    lights: Vec<usize>,
    proc_info: ProcessInfo,
    light_functions: LightFunctions,
    show: bool,
    display_key: util::KeyState
}


impl Context {
    fn new() -> Self {
        println!("Initializing");
        // hudhook::utils::alloc_console();
        let proc_info = ProcessInfo::new(None).unwrap();

        let memory_pool: usize =
            unsafe { *((proc_info.region.start_address + 0x2c46878) as *const usize) };
        // TODO: Check how to do this!!
        let world = 0x000001AF5F8F2590;
        let lights = Vec::new();
        let light_functions = LightFunctions::new(proc_info.region.start_address);

        Self {
            memory_pool,
            world,
            lights,
            light_functions,
            proc_info,
            show: false,
            display_key: util::KeyState::new(VK_F4 as _)
        }
    }

    // TODO: fix the position stuff.
    unsafe fn spawn_new_light(&mut self) {
        let light =
            (self.light_functions.ctor_caller)(self.memory_pool, 0, 1, std::ptr::null_mut());
        (*light).pos = Position {
            x: -403.67,
            y: -253.33,
            z: 8.0,
        };
        (*light).light.brightness = 1000.0;
        (*light).light.radius = 180.0;

        (self.light_functions.flag_setter)(light, self.world);
        (*light).shadow_casting_mode = 2;

        self.lights.push(light as usize);
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
            imgui::Slider::new("Brightness", f32::MIN, f32::MAX)
                .range(0.1, 10000.0)
                .build(&ui, &mut brightness);
            imgui::Slider::new("radius", f32::MIN, f32::MAX)
                .range(0.1, 360.0)
                .build(&ui, &mut radius);

            let mut position: [f32; 3] = light.pos.into();
            imgui::InputFloat3::new(&ui, "Position", &mut position)
                .no_horizontal_scroll(false)
                .build();

            light.pos = position.into();
            light.light.brightness = brightness;
            light.light.radius = radius;
        });
}

impl ImguiRenderLoop for Context {
    fn render(&mut self, ui: &mut imgui_dx11::imgui::Ui, flags: &ImguiRenderLoopFlags) {

        if flags.focused && !ui.io().want_capture_keyboard && self.display_key.keyup() {
            self.show = !self.show;
        }

        if self.show {
            ui.set_mouse_cursor(Some(imgui::MouseCursor::Arrow));
            Window::new("Main window")
                .size([200.0, 200.0], Condition::FirstUseEver)
                .build(ui, || {
                    if ui.button("Spawn new light") {
                        unsafe {
                            self.spawn_new_light();
                        }
                    }
                });

            for (i, light) in self.lights.iter_mut().enumerate() {
                let light = unsafe { &mut *(*light as *mut LightEntity) };
                render_window_per_light(ui, light, i);
                unsafe {
                    (self.light_functions.render_update)(light, self.world);
                }
            }
        } else {
            ui.set_mouse_cursor(None);
        }
    }
}

hudhook::hudhook!(Context::new().into_hook());
