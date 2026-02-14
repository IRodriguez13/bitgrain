use std::slice;

extern "C" {
    pub fn quantize_block(
        block: *mut i16,
        table: *const i16,
    );

    pub fn bitstream_write_byte(
        buffer: *mut u8,
        position: *mut i32,
        value: u8,
    );
}

/// Codifica imagen en escala de grises.
/// image: width*height bytes; out_buffer: buffer de salida; out_capacity: tamaño máximo.
/// Escribe en *out_len el número de bytes generados. Devuelve 0 ok, -1 error.
#[no_mangle]
pub extern "C" fn bitgrain_encode_grayscale(
    image: *const u8,
    width: u32,
    height: u32,
    out_buffer: *mut u8,
    out_capacity: u32,
    out_len: *mut i32,
) -> i32 {
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return -1;
    }
    let size = (width as usize).saturating_mul(height as usize);
    let image_slice = unsafe { slice::from_raw_parts(image, size) };
    let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
    let mut pos: i32 = 0;
    crate::encoder::encode_grayscale(image_slice, width as usize, height as usize, buffer_slice, &mut pos);
    unsafe { *out_len = pos };
    0
}

/// Decodifica un flujo .bg en imagen en escala de grises.
/// buffer/size: flujo .bg completo (con cabecera).
/// out_pixels: buffer de salida (width*height bytes); debe tener al menos out_capacity.
/// out_width, out_height: dimensiones de la imagen decodificada.
/// Devuelve 0 ok, -1 error.
#[no_mangle]
pub extern "C" fn bitgrain_decode_grayscale(
    buffer: *const u8,
    size: i32,
    out_pixels: *mut u8,
    out_capacity: u32,
    out_width: *mut u32,
    out_height: *mut u32,
) -> i32 {
    if buffer.is_null() || out_pixels.is_null() || out_width.is_null() || out_height.is_null() {
        return -1;
    }
    if size <= 0 || out_capacity == 0 {
        return -1;
    }
    let buf_slice = unsafe { slice::from_raw_parts(buffer, size as usize) };
    let out_slice = unsafe { slice::from_raw_parts_mut(out_pixels, out_capacity as usize) };
    let ok = crate::decoder::decode_grayscale(
        buf_slice,
        out_slice,
        unsafe { &mut *out_width },
        unsafe { &mut *out_height },
    );
    if ok { 0 } else { -1 }
}
