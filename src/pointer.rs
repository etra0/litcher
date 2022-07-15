use std::marker::PhantomData;

pub struct Pointer<T: 'static> {
    base_addr: usize,
    offsets: Vec<usize>,
    _marker: PhantomData<T>,
    pub last_value: Option<usize>,
    pub should_update: bool
}

impl<T: 'static> Pointer<T> {
    pub fn new(base_addr: usize, offsets: Vec<usize>) -> Self {
        Self {
            base_addr,
            offsets,
            _marker: PhantomData::default(),
            last_value: None,
            should_update: false
        }
    }

    pub unsafe fn read(&mut self) -> Option<&'static T> {
        let mut current_addr = std::ptr::read((self.base_addr) as *const usize);
        for offset in self.offsets.iter() {
            if current_addr == 0 {
                return None;
            }

            current_addr = std::ptr::read((current_addr + offset) as *const usize);
        }

        if let Some(lv) = self.last_value {
            if lv != current_addr {
                self.should_update = true;
                self.last_value = Some(current_addr);
            }
        } else {
            self.last_value = Some(current_addr);
        }

        Some(std::mem::transmute(current_addr))
    }

}
