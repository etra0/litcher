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
        let mp = generate_aob_pattern![0x48, 0x85, 0xD2, 0x74, 0x10, 0x4C, 0x39, 0x72, 0x08, 0x74, 0x0A, 0x48, 0x8B, 0x52, 0x08];

        let addr = proc_info.region.scan_aob(&mp).unwrap().unwrap();

        let mut detour = unsafe {
            Detour::new(addr, 32, &raw const overwrite_tonemapping as usize, Some(&mut overwrite_tonemapping_jmb))
        };

        detour.inject();
        Self {
            value: 1.0,
            overwrite: false,
            detour
        }
    }

    pub fn handle_ui(&mut self, ui: &imgui::Ui) {
        ui.slider("Exposure", 0.0, 3.0, &mut self.value);
        ui.same_line();
        ui.checkbox("Overwrite", &mut self.overwrite);

        unsafe {
            overwrite_tonemapping_enable = self.overwrite as u8;
            overwrite_tonemapping_val = self.value;
        }

    }
}
