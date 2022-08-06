//! Some things that are useful for debugging:
//! The render_proxy function is at [$process + 02a0f50]. It receives two arguments, the light
//! pointer and the world. It's useful to hook into it to steal the world pointer.
//! Offset of the CR4Player Memory Pool: [$process + 2d56848]
#![feature(once_cell)]

use imgui::ColorEditFlags;
use memory_rs::internal::{memory::resolve_module_path, process_info::ProcessInfo};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

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
    id_track: usize,
}

const VERSION: &'static str = concat!("The Litcher v", env!("CARGO_PKG_VERSION"), ", by @etra0");

impl Context {
    fn new() -> Self {
        println!("Initializing");

        if cfg!(debug_assertions) {
            hudhook::utils::alloc_console();
        }
        // hudhook::utils::simplelog();
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
            show: true,
            player: CR4Player::new(player),
            memory_pool_func,
            id_track: 0,
        }
    }

    pub fn get_pos_rot(&self) -> Option<(Position, RotationMatrix)> {
        let camera = self.player.get_camera()?;
        let pos = camera.pos;
        let rot = camera.rot_matrix;
        Some((pos, rot))
    }

    pub fn main_window(&mut self, ui: &mut imgui::Ui) {
        Window::new(VERSION)
            .size([410.0, 200.0], Condition::FirstUseEver)
            .build(ui, || {
                if ui.button("Spawn new pointlight") {
                    if let (Some((pos, rot)), Some(world)) =
                        (self.get_pos_rot(), self.player.get_world())
                    {
                        unsafe {
                            let light = LightContainer::new(
                                LightType::PointLight(PointLight::new(
                                    self.memory_pools.pointlight.read().unwrap(),
                                    self.memory_pool_func,
                                    pos,
                                    rot,
                                    world,
                                )),
                                self.id_track,
                            );
                            self.id_track += 1;
                            self.lights.push(light);
                        }
                    }
                }

                if ui.button("Spawn new spotlight") {
                    if let (Some((pos, rot)), Some(world)) =
                        (self.get_pos_rot(), self.player.get_world())
                    {
                        unsafe {
                            let light = LightContainer::new(
                                LightType::SpotLight(SpotLight::new(
                                    self.memory_pools.spotlight.read().unwrap(),
                                    self.memory_pool_func,
                                    pos,
                                    rot,
                                    world,
                                )),
                                self.id_track,
                            );
                            self.id_track += 1;
                            self.lights.push(light);
                        }
                    }
                }

                ui.separator();

                let world = self.player.get_world();
                if world.is_none() {
                    return;
                }

                let world = world.unwrap();

                let mut light_to_remove = None;
                self.lights.iter_mut().enumerate().for_each(|(i, light)| {
                    let id = ui.push_id(&light.id);
                    // TODO: maybe remove allocations from here, lol
                    if ui.button("X") {
                        light_to_remove = Some(i);
                    }
                    ui.same_line();
                    ui.text(&light.id);
                    ui.same_line();
                    imgui::ColorButton::new("Color of light ##", light.color)
                        .flags(ColorEditFlags::NO_INPUTS | ColorEditFlags::NO_LABEL)
                        .build(ui);
                    ui.same_line();
                    if ui.button("Edit") {
                        light.open = true;
                    }
                    ui.same_line();
                    if ui.button("Toggle on/off") {
                        light.toggle(world);
                    }

                    ui.same_line();
                    ui.checkbox("Attach to camera", &mut light.attach_camera);

                    id.end();
                });

                if let Some(ix) = light_to_remove {
                    println!("Light to remove: {}", ix);
                    let light = self.lights.remove(ix);
                    light.remove_light(world);
                }
            });
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

        if cfg!(debug_assertions) && ui.is_key_index_pressed_no_repeat(VK_F6 as _) {
            hudhook::lifecycle::eject();
        }

        if ui.is_key_index_pressed_no_repeat(VK_F5 as _) {
            // Before we do the clear, let's just deactivate all the current lights.
            if let Some(world) = self.player.get_world() {
                self.lights.drain(..).for_each(|light| {
                    light.remove_light(world);
                });
                self.player.updated();
            }
        }

        self.lights.retain(|x: &LightContainer| {
            let ptr = match &x.light {
                LightType::PointLight(PointLight { light, .. }) => light,
                LightType::SpotLight(SpotLight { light, .. }) => light,
            };
            !ptr.should_get_deleted()
        });

        if self.show {
            ui.set_mouse_cursor(Some(imgui::MouseCursor::Arrow));
            self.main_window(ui);

            self.lights.iter_mut().for_each(|lw| lw.render_window(ui));
        } else {
            ui.set_mouse_cursor(None);
        }

        if let (Some((pos, rot)), Some(world)) = (self.get_pos_rot(), self.player.get_world()) {
            for light_wrapper in self.lights.iter_mut() {
                if light_wrapper.attach_camera {
                    light_wrapper.set_pos_rot(pos, rot);
                }

                match &mut light_wrapper.light {
                    LightType::PointLight(pl) => {
                        pl.update_render(world);
                    }
                    LightType::SpotLight(spl) => {
                        spl.update_render(world);
                    }
                };
            }
        }
    }

    fn should_block_messages(&self, _io: &imgui::Io) -> bool {
        self.show
    }
}

hudhook::hudhook!(Context::new().into_hook::<ImguiDX11Hooks>());
