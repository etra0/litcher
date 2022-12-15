//! Some things that are useful for debugging:
//! The render_proxy function is at [$process + 02a0f50]. It receives two arguments, the light
//! pointer and the world. It's useful to hook into it to steal the world pointer.
//! Offset of the CR4Player Memory Pool: [$process + 2d56848]
#![feature(once_cell)]

use std::panic::PanicInfo;

use anyhow::{Context, Result};
use imgui::ColorEditFlags;
use memory_rs::generate_aob_pattern;
use memory_rs::internal::process_info::ProcessInfo;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::hooks::{ImguiRenderLoop, ImguiRenderLoopFlags};
use imgui::{Condition, Window};

mod definitions;
mod pointer;

use definitions::*;
use pointer::*;
use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxA;

struct LitcherContext {
    memory_pools: MainMemoryPools,
    lights: Vec<LightContainer>,
    show: bool,
    player: CR4Player,
    id_track: usize,
}

const VERSION: &str = concat!("The Litcher v", env!("CARGO_PKG_VERSION"), ", by @etra0");

fn panic(info: &PanicInfo) {
    let msg = format!(
        "Something went super wrong.\n\n\
        Please post the log created alongside witcher3.exe on a github issue in \
        https://github.com/etra0/litcher.\n\
        We got a panic with the following information:\n\n\
        {:#?}\n\
        It is suggested to restart the game since anything can happen anyway.\n\
        This is totally unexpected behavior for the creator.
        \0",
        info
    );
    unsafe { MessageBoxA(0, msg.as_ptr(), "The Litcher\0".as_ptr(), 0) };
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1000));
    }
}

impl LitcherContext {
    fn new() -> Self {
        println!("Initializing");
        std::panic::set_hook(Box::new(panic));

        if cfg!(debug_assertions) {
            hudhook::utils::alloc_console();
        }
        // hudhook::utils::simplelog();
        let proc_info = ProcessInfo::new(None).unwrap();

        let memory_pools = Self::find_memory_pools(&proc_info).unwrap();

        let player: Pointer<ScriptedEntity<EmptyVT>> = {
            let mp = generate_aob_pattern![
                0xBA, 0x10, 0x00, 0x00, 0x00, 0x48, 0x89, 0x9C, 0x24, 0xB0, 0x00, 0x00, 0x00, 0x48, 0x8B, 0x1D
            ];

            let instr = proc_info
                .region
                .scan_aob(&mp)
                .context("Couldn't find Player entity")
                .unwrap()
                .unwrap() + 13;

            let player_entity = unsafe { Self::read_from_mov(instr) };

            Pointer::new(player_entity, vec![0x1A8, 0x40])
        };

        let lights = Vec::new();

        Self {
            memory_pools,
            lights,
            show: true,
            player: CR4Player::new(player),
            id_track: 0,
        }
    }

    unsafe fn read_from_mov(addr: usize) -> usize {
        let instruction_length = 7_usize;

        // Basically, the `mov` instruction works with offsets, in this case,
        // we know the length of the mov instruction, which is 3 bytes and then
        // the offset. We need to skip the first three bytes, read that offset,
        // then add the instruction length itself because the offset is
        // calculated *after* the instruction is read.
        // Basically, (RIP + instr_length) + offset
        let offset = *((addr + 0x3) as *const u32) as usize;

        // Finally, the *real* address would be
        //   instr + instruction_length + offset
        (addr + instruction_length) + offset
    }

    /// Find the memory pools of the game doing pointer trickery.
    /// Since they're global variables, there are more than one place where
    /// they have references to it. We can safely assume those instructions
    /// *will* exist in both GOG's and Steam's version, so we can avoid having
    /// hardcoded offsets.
    fn find_memory_pools(proc_info: &ProcessInfo) -> Result<MainMemoryPools> {
        let region = &proc_info.region;
        // Memory pattern for PointLightComponent memory pool.
        // NOTE: One easy trick to find them is to look at what references are for the Builder
        // class, from that you can find the refernece to the Memory Pool then build an AOB from a
        // function.
        let mp = generate_aob_pattern![
            0x0F, 0x11, 0x83, 0x08, 0x02, 0x00, 0x00, 0x4C, 0x8B, 0x07, 0x4D, 0x85, 0xC0, 0x74, 0x2A, 0x48, 0x8B, 0x0D
        ];

        // The right instruction is in 0x10 offset from what we find.
        let instr = region
            .scan_aob(&mp)?
            .context("Couldn't find the PointLight Memory Pool")?
            + 15;

        let point_light_memorypool = unsafe { Self::read_from_mov(instr) };
        println!("PointLight memorypool: {:x}", point_light_memorypool);

        // Now we try to find the SpotLightComponent
        let mp = generate_aob_pattern![
            0x32, 0xC2, 0x88, 0x87, 0xB4, 0x01, 0x00, 0x00, 0xEB, _, 0x48, 0x8B, 0x05
        ];

        let instr = region
            .scan_aob(&mp)?
            .context("Couldn't find SpotLightComponent Memory Pool")?
            + 10;

        let spot_light_memorypool = unsafe { Self::read_from_mov(instr) };
        println!("SpotLight memorypool: {:x}", spot_light_memorypool);

        Ok(MainMemoryPools {
            spotlight: Pointer::new(spot_light_memorypool, Vec::new()),
            pointlight: Pointer::new(point_light_memorypool, Vec::new()),
        })
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

                let world = if let Some(world) = self.player.get_world() {
                    world
                } else {
                    return;
                };

                let mut light_to_remove = None;
                self.lights.iter_mut().enumerate().for_each(|(i, light)| {
                    let id = ui.push_id(&light.id);
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

impl ImguiRenderLoop for LitcherContext {
    fn render(&mut self, ui: &mut imgui::Ui, flags: &ImguiRenderLoopFlags) {
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

hudhook::hudhook!(LitcherContext::new().into_hook::<ImguiDx12Hooks>());
