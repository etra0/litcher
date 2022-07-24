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
    lights: Vec<LightContainer>,
    show: bool,
    player: CR4Player,
    memory_pool_func: MemoryPoolFunc,
    base_addr: usize,
}

impl Context {
    fn new() -> Self {
        println!("Initializing");
        // hudhook::utils::alloc_console();
        let proc_info = ProcessInfo::new(None).unwrap();

        let memory_pools = MainMemoryPools {
            pointlight: Pointer::new(proc_info.region.start_address + 0x2c46878, Vec::new()),
            spotlight: Pointer::new(proc_info.region.start_address + 0x2c46900, Vec::new()),
        };

        // Another option:
        // [$process + 0x2c5dd38]
        // [0x1A8, 0x40] <= CR4Player
        let player: Pointer<ScriptedEntity<EmptyVT>> = Pointer::new(
            proc_info.region.start_address + 0x2c5dd38,
            vec![0x1A8, 0x40],
        );

        let lights = Vec::new();

        let memory_pool_func: MemoryPoolFunc =
            unsafe { std::mem::transmute(proc_info.region.start_address + 0x03c400) };

        Self {
            memory_pools,
            lights,
            show: false,
            player: CR4Player::new(player),
            memory_pool_func,
            base_addr: proc_info.region.start_address,
        }
    }

    // This has to be mutable because anything we get from the self player when we do a get
    // actually mutates the player itself. Maybe at some point we should use some sort of RefCell
    // since the player as it is doesn't *actually* change.
    pub fn get_pos_rot(&mut self) -> Option<(Position, RotationMatrix)> {
        let camera = self.player.get_camera2()?;
        let pos = camera.pos;
        let rot = camera.rot_matrix;
        Some((pos, rot))
    }
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

        if ui.is_key_index_pressed_no_repeat(VK_F5 as _) {
            // Before we do the clear, let's just deactivate all the current lights.
            if let Some(world) = self.player.get_world() {
                for light_wrapper in self.lights.iter_mut() {
                    match &mut light_wrapper.light {
                        LightType::SpotLight(l) => {
                            l.light.is_enabled = false;
                            l.update_render(world);
                        }
                        LightType::PointLight(l) => {
                            l.light.is_enabled = false;
                            l.update_render(world);
                        }
                    }
                }
                self.lights.clear();
                self.player.updated();
            }
        }

        if self.show {
            ui.set_mouse_cursor(Some(imgui::MouseCursor::Arrow));
            Window::new("Main window")
                .size([200.0, 200.0], Condition::FirstUseEver)
                .build(ui, || {

                    if ui.button("Spawn new pointlight") {
                        if let (Some((pos, rot)), Some(world)) = (self.get_pos_rot(), self.player.get_world()) {
                            unsafe {
                                let light = LightContainer::new(LightType::PointLight(PointLight::new(self.memory_pools.pointlight.read().unwrap(), self.memory_pool_func, pos, rot, world)));
                                self.lights.push(light);
                            }
                        }
                    }

                    if ui.button("Spawn new spotlight") {
                        if let (Some((pos, rot)), Some(world)) = (self.get_pos_rot(), self.player.get_world()) {
                            unsafe {
                                let light = LightContainer::new(LightType::SpotLight(SpotLight::new(self.memory_pools.spotlight.read().unwrap(), self.memory_pool_func, pos, rot, world)));
                                self.lights.push(light);
                            }
                        }
                    }
                });

            self.lights.retain(|x: &LightContainer| {
                let ptr = match &x.light {
                    LightType::PointLight(PointLight { light, .. }) => light,
                    LightType::SpotLight(SpotLight { light, .. }) => light,
                };
                !ptr.should_get_deleted()
            });

            if let (Some((pos, rot)), Some(world)) = (self.get_pos_rot(), self.player.get_world()) {
                let lights_iter = self.lights.iter_mut();
                for (i, light_wrapper) in lights_iter.enumerate() {
                    light_wrapper.render_window(ui, i);

                    match &mut light_wrapper.light {
                        LightType::PointLight(pl) => {
                            pl.update_render(world);
                        },
                        LightType::SpotLight(spl) => {
                            spl.update_render(world);
                        },
                    };

                    if light_wrapper.attach_camera {
                        light_wrapper.set_pos_rot(pos, rot);
                    }
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
