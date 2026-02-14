#[derive(Clone, Copy)]
pub struct Block {
    pub data: [i16; 64],
}

impl Block {
    pub fn new() -> Self {
        Self { data: [0; 64] }
    }

    pub fn from_slice(slice: &[i16]) -> Self {
        assert!(slice.len() == 64);

        let mut data = [0i16; 64];
        data.copy_from_slice(slice);

        Self { data }
    }

    pub fn as_mut_ptr(&mut self) -> *mut i16 {
        self.data.as_mut_ptr()
    }

    pub fn as_ptr(&self) -> *const i16 {
        self.data.as_ptr()
    }
}

