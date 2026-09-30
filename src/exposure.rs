use lazy_re::lazy_re;
use memory_rs::generate_aob_pattern;
use memory_rs::internal::injections::{Inject, Detour};
use memory_rs::internal::process_info::ProcessInfo;

memory_rs::scoped_no_mangle! {
    overwrite_tonemapping_jmb: usize = 0x0;
    overwrite_tonemapping_val: f32 = 1.0;
    overwrite_tonemapping_enable: u8 = 0x0;
}

extern "C" {
    pub static overwrite_tonemapping: u8;
}

pub struct ToneMappingContainer {
    detour: Detour,
    value: f32,
    overwrite: bool
}

impl ToneMappingContainer {
    pub fn new(proc_info: &ProcessInfo) -> Self {
        let mp = generate_aob_pattern![
            0x49, 0x8B, 0x86, 0x20, 0x02, 0x00, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x0D, 0x48, 0x8B, 0x48, 0x08, 0x48, 0x8D, 0x51, 0x70, 0x48, 0x85, 0xC9
        ];

        let addr = proc_info.region.scan_aob(&mp).unwrap().unwrap() + 0x7;

        let mut detour = unsafe {
            Detour::new(addr, 28, &raw const overwrite_tonemapping as usize, Some(&mut overwrite_tonemapping_jmb))
        };

        detour.inject();
        Self {
            value: 1.0,
            overwrite: false,
            detour
        }
    }

    pub fn handle_ui(&mut self, ui: &imgui::Ui) {
        ui.slider_config("Exposure", 1e-6, 3.0).flags(imgui::SliderFlags::LOGARITHMIC).build(&mut self.value);
        ui.checkbox("Overwrite", &mut self.overwrite);
        ui.separator();

        unsafe {
            overwrite_tonemapping_enable = self.overwrite as u8;
            overwrite_tonemapping_val = self.value;
        }

    }
}
