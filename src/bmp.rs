//! archive
mod bmp {
    use esp_println::println;

    use crate::lcd::Rgb16;

    /// Extract the data of a `bmp` image. The assumption is that you know the
    /// format already, and you just neeed the data. Panics if the BMP is not valid.
    pub fn bmp_data(bytes: &[u8]) -> &[u8] {
        // BMP is a little-endian format
        let offset = u32::from_le_bytes(
            bytes[10..14]
                .try_into()
                .expect("caller asserts this is valid bmp, which always contains a 14 byte header"),
        ) as usize;
        &bytes[offset..]
    }

    #[repr(C)]
    #[derive(Debug)]
    pub struct Rgba(u8, u8, u8, u8);

    impl Rgba {
        /// Converts to RGB565, ignoring alpha completely
        pub fn to_rgb16(&self) -> Rgb16 {
            if self.3 == 0 {
                return Rgb16::IGNORE;
            }

            Rgb16::from_rgb(self.0, self.1, self.2)
        }
    }

    pub fn bytes_as_rgba(bytes: &[u8]) -> &[Rgba] {
        unsafe { core::slice::from_raw_parts(bytes as *const [u8] as _, bytes.len() / 4) }
    }

    pub fn bytes_as_rgb16(bytes: &[u8], buf: &mut [Rgb16]) {
        let rgbas = bytes_as_rgba(bytes);
        for (i, color) in rgbas.iter().enumerate() {
            println!("{i}");
            buf[i] = color.to_rgb16();
        }
    }
}