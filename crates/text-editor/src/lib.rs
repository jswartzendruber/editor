use crop::{Rope, RopeBuilder, RopeSlice};

/// Contains information needed to lay out a glyph on the screen.
/// https://freetype.org/freetype2/docs/glyphs/glyphs-3.html
/// See diagram in section 3.
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub advance: (f32, f32),
    pub size: (f32, f32),
    pub pos: (f32, f32),
}

pub trait GlyphRasterizer {
    /// Get the metrics from the given character and font size.
    fn get_glyph(&mut self, c: char, font_size: f32) -> GlyphMetrics;
}

#[derive(Debug)]
pub enum ScrollAmount {
    Up { lines: usize },
    Down { lines: usize },
    ToStart,
    ToEnd,
}

#[derive(Debug)]
pub struct TextEditor {
    /// Contains all of the text inside of this text editor.
    content: Rope,

    /// The current position of the cursor in the text rope.
    cursor_position: usize,

    /// The starting index of the text that will be rendered.
    text_start_idx: usize,

    /// The current font size
    font_size: f32,

    /// Window width in pixels
    window_width: f32,

    /// Window height in pixels
    window_height: f32,
}

impl TextEditor {
    /// Creates a text editor using the given content, which will wrap
    /// whenever lines exceed 'wrap_at' characters per line.
    pub fn new(content: &str, window_width: f32, window_height: f32, font_size: f32) -> Self {
        let mut builder = RopeBuilder::new();
        builder.append(content);

        let text = builder.build();

        Self {
            content: text,
            cursor_position: 0,
            text_start_idx: 0,
            font_size,
            window_width,
            window_height,
        }
    }

    pub fn update_window_size(&mut self, new_width: f32, new_height: f32) {
        self.window_width = new_width;
        self.window_height = new_height;
    }

    /// Get the current position of the cursor
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// This function will use the glyph metrics to decide when to wrap characters.
    /// A line ends if:
    ///  - A newline character is reached, or
    ///  - We cannot fit any more characters on the current line, or
    ///  - We reach the end of the internal character rope
    pub fn layout_lines(&self, glyph_rasterizer: &mut impl GlyphRasterizer) -> Vec<RopeSlice<'_>> {
        let mut lines = vec![];
        let line_height = self.font_size * 1.2;
        let start_index = self.text_start_idx;

        let mut curr_line_start_index = start_index;
        let mut byte_index = start_index;
        let mut x = 0.0;
        let mut y = 0.0;
        for c in self.content.byte_slice(start_index..).chars() {
            // We've reached the end of this line, save the offsets
            if c == '\n' {
                lines.push(self.content.byte_slice(curr_line_start_index..byte_index));
                byte_index += c.len_utf8();
                curr_line_start_index = byte_index;
                y += line_height;
                x = 0.0;
                continue;
            }

            let glyph_metrics = glyph_rasterizer.get_glyph(c, self.font_size);

            if x + glyph_metrics.advance.0 >= self.window_width {
                lines.push(self.content.byte_slice(curr_line_start_index..byte_index));
                curr_line_start_index = byte_index;
                y += line_height;
                x = 0.0;
            } else if y + glyph_metrics.advance.1 >= self.window_height {
                // We are done!
                break;
            }

            x += glyph_metrics.advance.0;
            y += glyph_metrics.advance.1;

            byte_index += c.len_utf8();
        }

        lines
    }

    pub fn backspace(&mut self) {
        let len = self.content.byte_len();
        if len == 0 {
            return;
        }

        self.content.delete(len - 1..len);
        self.cursor_position -= 1;
    }

    pub fn add_char(&mut self, c: &str) {
        self.content.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn scroll(&mut self, scroll: ScrollAmount) {
        match scroll {
            ScrollAmount::Up { lines } => self.scroll_up(lines),
            ScrollAmount::Down { lines } => self.scroll_down(lines),
            ScrollAmount::ToStart => self.scroll_to_start(),
            ScrollAmount::ToEnd => self.scroll_to_end(),
        }
    }

    fn scroll_to_start(&mut self) {
        self.text_start_idx = 0;
        self.cursor_position = 0;
    }

    fn scroll_to_end(&mut self) {
        let bottom = self.content.byte_len().saturating_sub(1);

        self.text_start_idx = bottom;
        self.cursor_position = bottom;
        self.scroll_up(1); // so we aren't totally at the bottom and see nothing
    }

    /// Scroll the viewport up 'lines' wrapped lines.
    fn scroll_up(&mut self, lines: usize) {
        todo!();
        // let mut byte_index = self.text_start_idx;

        // // Detect infinite loops when fuzzing.
        // // Not that there are any...
        // let mut loop_fuel = 10000;

        // let mut lines_passed = 0;

        // while lines_passed < lines {
        //     let mut char_count = 0;
        //     while byte_index > 0 {
        //         // Find the character boundary
        //         while (!self.content.is_char_boundary(byte_index) && byte_index > 0)
        //             || byte_index >= self.content.byte_len()
        //         {
        //             byte_index -= 1;
        //         }

        //         // See if we have a newline and found the end of the line, or
        //         // if we are at our wrap limit.
        //         // let c = self.content.byte(byte_index);
        //         // while c == b'\n'
        //         //     && byte_index <= self.content.byte_len()
        //         //     && self.content.is_char_boundary(byte_index)
        //         // {
        //         //     byte_index -= 1;
        //         //     break 'line_loop;
        //         // }
        //         if char_count == self.wrap_at {
        //             break;
        //         }

        //         // Include the current character in our set. Update offsets.
        //         char_count += 1;
        //         byte_index = byte_index.saturating_sub(1);

        //         loop_fuel -= 1;
        //         if loop_fuel <= 0 {
        //             panic!("layout_line_naive infinite loop");
        //         }
        //     }

        //     lines_passed += 1;
        // }

        // self.cursor_position = byte_index;
        // self.text_start_idx = byte_index;
    }

    /// Scroll the viewport down 'lines' wrapped lines.
    fn scroll_down(&mut self, lines: usize) {
        todo!();
        // let mut byte_index = self.text_start_idx;

        // // Detect infinite loops when fuzzing.
        // // Not that there are any...
        // let mut loop_fuel = 10000;

        // let mut lines_passed = 0;

        // 'outer: while lines_passed < lines {
        //     let mut char_count = 0;
        //     loop {
        //         // Find the character boundary
        //         while !self.content.is_char_boundary(byte_index)
        //             && byte_index < self.content.byte_len()
        //         {
        //             byte_index += 1;
        //         }

        //         // If we are at the end of the string, we need to cap our
        //         // index at the end of the string and bail
        //         if byte_index >= self.content.byte_len() {
        //             byte_index = self.content.byte_len().saturating_sub(1);
        //             break 'outer;
        //         }

        //         // See if we have a newline and found the end of the line, or
        //         // if we are at our wrap limit.
        //         let c = self.content.byte(byte_index);
        //         if c == b'\n' || char_count == self.wrap_at {
        //             break;
        //         }

        //         // Include the current character in our set. Update offsets.
        //         char_count += 1;
        //         byte_index += 1;

        //         loop_fuel -= 1;
        //         if loop_fuel <= 0 {
        //             panic!("layout_line_naive infinite loop");
        //         }
        //     }

        //     // skip past the newline character if there was one
        //     while byte_index + 1 < self.content.byte_len()
        //         && self.content.is_char_boundary(byte_index)
        //         && self.content.byte(byte_index) == b'\n'
        //     {
        //         byte_index += 1;
        //     }

        //     lines_passed += 1;
        // }

        // self.cursor_position = byte_index;
        // self.text_start_idx = byte_index;
    }
}
