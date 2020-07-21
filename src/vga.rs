use core::fmt::{Error, Write};
use core::ptr;
use core::slice;
use lazy_static::lazy_static;
use spin::Mutex;

const SCREEN_WIDTH: usize = 80;
const SCREEN_HEIGHT: usize = 25;

lazy_static! {
    pub static ref VGA_WRITER: Mutex<VgaWriter<'static>> = {
        let vga = Mutex::new(VgaWriter::new(unsafe {
            slice::from_raw_parts_mut(0xb8000 as *mut VgaChar, SCREEN_WIDTH * SCREEN_HEIGHT)
        }));
        vga.lock().clear();
        vga
    };
}

#[repr(u8)]
#[allow(dead_code)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xA,
    LightCyan = 0xB,
    LightRed = 0xC,
    LightMagenta = 0xD,
    Yellow = 0xE,
    White = 0xF,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | foreground as u8)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct VgaChar {
    character: u8,
    color_code: ColorCode,
}

impl VgaChar {
    pub fn new(character: u8, color_code: ColorCode) -> VgaChar {
        VgaChar {
            character,
            color_code,
        }
    }
}

// TODO: Refactor [VgaChar] to a VgaBuffer type that allows us to use 2d indexing
pub struct VgaWriter<'a> {
    row: usize,
    col: usize,
    color: ColorCode,
    buffer: &'a mut [VgaChar],
}

impl<'a> VgaWriter<'a> {
    fn new(buffer: &'a mut [VgaChar]) -> VgaWriter<'a> {
        // ensure the slice is large enough
        assert_eq!(buffer.as_mut().len(), SCREEN_WIDTH * SCREEN_HEIGHT);

        let row = 0;
        let col = 0;
        let color = ColorCode::new(Color::White, Color::Black);

        VgaWriter {
            row,
            col,
            color,
            buffer,
        }
    }

    pub fn clear(&mut self) {
        let blank_char = VgaChar::new(b' ', self.color);

        for row in 0..SCREEN_HEIGHT {
            for col in 0..SCREEN_WIDTH {
                unsafe {
                    let pointer = &mut self.buffer[row * SCREEN_WIDTH + col] as *mut VgaChar;
                    ptr::write_volatile(pointer, blank_char);
                }
            }
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        // TODO: clean up this method
        if byte == b'\n' {
            self.row += 1;
            if self.row == SCREEN_HEIGHT {
                self.scroll();
            }
            self.col = 0;
            return;
        }

        let vga_char = VgaChar::new(byte, self.color);

        unsafe {
            let pointer = &mut self.buffer[self.row * SCREEN_WIDTH + self.col] as *mut VgaChar;
            ptr::write_volatile(pointer, vga_char);
        }

        self.col += 1;

        if self.col == SCREEN_WIDTH {
            self.row += 1;
            if self.row == SCREEN_HEIGHT {
                self.scroll();
            }
            self.col = 0;
        }
    }

    fn scroll(&mut self) {
        let slice = self.buffer.as_mut();
        for row in 1..SCREEN_HEIGHT {
            let src = &mut slice[row * SCREEN_WIDTH] as *mut VgaChar;
            let dst = &mut slice[(row - 1) * SCREEN_WIDTH] as *mut VgaChar;

            // SAFETY: We know that each row is non-overlapping, as they are separate logical rows.
            //         Similarly, there is SCREEN_WIDTH * size_of::<VgaChar> bytes in the row due
            //         to the math above. Lastly, VgaChar is copy, so this is okay.
            unsafe {
                ptr::copy_nonoverlapping(src, dst, SCREEN_WIDTH);
            }
        }

        let blank_char = VgaChar::new(b' ', self.color);

        for col in 0..SCREEN_WIDTH {
            let pointer = &mut slice[(SCREEN_HEIGHT - 1) * SCREEN_WIDTH + col] as *mut VgaChar;

            // SAFETY: The ponter is valid and properly aligned, as we simply get the address of
            //         the existing slice above.
            unsafe {
                ptr::write_volatile(pointer, blank_char);
            }
        }

        self.row = SCREEN_HEIGHT - 1;
        self.col = 0;
    }
}

impl<'a> Write for VgaWriter<'a> {
    fn write_str(&mut self, string: &str) -> Result<(), Error> {
        for byte in string.bytes() {
            match byte {
                b' '..=b'~' | b'\n' => self.write_byte(byte),
                _ => self.write_byte(254),
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn check_string_is_displayed(buffer: &[VgaChar], string: &str) {
        let mut curr_row = 0;
        let mut curr_column = 0;
        for index in 0..string.len() {
            if string.as_bytes()[index] == b'\n' {
                curr_row += 1;
                curr_column = 0;
                continue;
            }

            let expected_char = VgaChar::new(
                string.as_bytes()[index],
                ColorCode::new(Color::White, Color::Black),
            );

            assert_eq!(buffer[curr_column + curr_row * SCREEN_WIDTH], expected_char);
            curr_column += 1;

            if curr_column == SCREEN_WIDTH {
                curr_column = 0;
                curr_row += 1;
            }
        }
    }

    #[test]
    fn write_byte() {
        let mut memory = [VgaChar::new(b' ', ColorCode::new(Color::Black, Color::White));
            SCREEN_WIDTH * SCREEN_HEIGHT];
        let mut vga = VgaWriter::new(&mut memory[..]);

        vga.write_byte(b'c');
        let expected_char = VgaChar::new(b'c', ColorCode::new(Color::White, Color::Black));

        assert_eq!(memory[0], expected_char);
    }

    #[test]
    fn write_string() {
        let mut memory = [VgaChar::new(b' ', ColorCode::new(Color::Black, Color::White));
            SCREEN_WIDTH * SCREEN_HEIGHT];
        let mut vga = VgaWriter::new(&mut memory[..]);

        let string = "Hello World!";
        vga.write_str(string)
            .expect("write_str should never return an error!");

        check_string_is_displayed(&memory, &string);
    }

    #[test]
    fn new_lines() {
        let mut memory = [VgaChar::new(b' ', ColorCode::new(Color::Black, Color::White));
            SCREEN_WIDTH * SCREEN_HEIGHT];
        let mut vga = VgaWriter::new(&mut memory[..]);

        let string = "Hello World!\nGoodbye World!";
        vga.write_str(string)
            .expect("write_str should never return an error!");

        check_string_is_displayed(&memory, &string);
    }

    #[test]
    fn line_wrap() {
        let mut memory = [VgaChar::new(b' ', ColorCode::new(Color::Black, Color::White));
            SCREEN_WIDTH * SCREEN_HEIGHT];
        let mut vga = VgaWriter::new(&mut memory[..]);

        let string = "a".repeat(30);
        vga.write_str(&string)
            .expect("write_str should never return an error!");

        check_string_is_displayed(&memory, &string);
    }

    #[test]
    fn scrolling() {
        let mut memory = [VgaChar::new(b' ', ColorCode::new(Color::Black, Color::White));
            SCREEN_WIDTH * SCREEN_HEIGHT];
        let mut vga = VgaWriter::new(&mut memory[..]);

        let string = "Hello World!\nGoodbye World!\n".repeat(13);
        vga.write_str(&string)
            .expect("write_str should never return an error!");

        check_string_is_displayed(&memory, &string[28..]);
    }

    #[test]
    fn invalid_char() {
        let mut memory = [VgaChar::new(b' ', ColorCode::new(Color::Black, Color::White));
            SCREEN_WIDTH * SCREEN_HEIGHT];
        let mut vga = VgaWriter::new(&mut memory[..]);

        let string = "😀😃😄";
        vga.write_str(&string)
            .expect("write_str should never return an error!");

        let invalid_char = VgaChar::new(254, ColorCode::new(Color::White, Color::Black));
        for index in 0..string.len() {
            assert_eq!(memory[index], invalid_char);
        }
    }
}
