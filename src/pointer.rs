use std::marker::PhantomData;
use std::sync::Mutex;

pub struct Pointer<T: 'static> {
    base_addr: usize,
    offsets: Vec<usize>,
    _marker: PhantomData<T>,
    pub last_value: Mutex<Option<usize>>,
    pub should_update: Mutex<bool>
}

impl<T: 'static> Pointer<T> {
    pub fn new(base_addr: usize, offsets: Vec<usize>) -> Self {
        Self {
            base_addr,
            offsets,
            _marker: PhantomData::default(),
            last_value: Mutex::new(None),
            should_update: Mutex::new(false)
        }
    }

    pub unsafe fn read(&self) -> Option<&'static mut T> {
        let mut current_addr = std::ptr::read((self.base_addr) as *const usize);
        for offset in self.offsets.iter() {
            if current_addr == 0 {
                return None;
            }

            current_addr = std::ptr::read((current_addr + offset) as *const usize);
        }

        let mut last_value = self.last_value.lock().unwrap();
        if let Some(lv) = *last_value {
            if lv != current_addr {
                *last_value = Some(current_addr);
                *self.should_update.lock().unwrap() = true;
            }
        } else {
            *last_value = Some(current_addr);
        }

        Some(std::mem::transmute(current_addr))
    }

}
