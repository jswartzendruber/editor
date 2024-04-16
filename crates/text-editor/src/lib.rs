use copypasta::{ClipboardContext, ClipboardProvider};
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

    /// Is the control key currently pressed?
    pub ctrl_down: bool,

    /// Handle to the system clipboard for copy/paste
    clipboard_context: ClipboardContext,
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
            ctrl_down: false,
            clipboard_context: ClipboardContext::new().unwrap(),
        }
    }

    pub fn update_window_size(&mut self, new_width: f32, new_height: f32) {
        self.window_width = new_width;
        self.window_height = new_height;
    }

    pub fn update_font_size(&mut self, new_font_size: f32) {
        self.font_size = new_font_size;
    }

    /// Get the current position of the cursor
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Get the starting position of the text area that will be rendered
    pub fn text_start_idx(&self) -> usize {
        self.text_start_idx
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

        let mut byte_index = start_index;
        let mut y = 0.0;
        loop {
            let (has_trailing_newline, line) = self.layout_line(byte_index, glyph_rasterizer);
            byte_index += line.byte_len();
            lines.push(line);
            y += line_height;

            if has_trailing_newline {
                byte_index += 1;
            }

            if y >= self.window_height {
                // We are done!
                break;
            }
        }

        lines
    }

    /// This function will use the glyph metrics to decide when to wrap characters.
    /// The line ends if:
    ///  - A newline character is reached, or
    ///  - We cannot fit any more characters on the current line, or
    ///  - We reach the end of the internal character rope
    ///
    /// We assume that the start of the line is pixel 0, and it ends at pixel 'self.window_width'
    ///
    /// Returns: bool: If there is a trailing newline that needs to be consumed
    ///          RopeSlice: the content of this line
    fn layout_line(
        &self,
        start_index: usize,
        glyph_rasterizer: &mut impl GlyphRasterizer,
    ) -> (bool, RopeSlice<'_>) {
        let mut byte_index = start_index;
        let mut x = 0.0;
        for c in self.content.byte_slice(start_index..).chars() {
            // We've reached the end of this line, save the offsets
            if c == '\n' {
                return (true, self.content.byte_slice(start_index..byte_index));
            }

            let glyph_metrics = glyph_rasterizer.get_glyph(c, self.font_size);

            if x + glyph_metrics.advance.0 >= self.window_width {
                return (false, self.content.byte_slice(start_index..byte_index));
            }

            x += glyph_metrics.advance.0;
            byte_index += c.len_utf8();
        }

        // If we haven't returned yet, this is probably the last line
        (false, self.content.byte_slice(start_index..))
    }

    // Lays out the line in before the one we are on (from start_index). Primarily used for scrolling up.
    fn layout_line_rev(
        &self,
        start_index: usize,
        glyph_rasterizer: &mut impl GlyphRasterizer,
    ) -> (bool, RopeSlice<'_>) {
        let mut byte_index = start_index;
        let mut x = self.window_width;
        for c in self.content.byte_slice(..start_index).chars().rev() {
            if c == '\n' && byte_index == start_index {
                byte_index = byte_index.saturating_sub(1);
                continue;
            } else if c == '\n' {
                // We've reached the start of this line, save the offsets
                return (true, self.content.byte_slice(byte_index..start_index));
            }

            let glyph_metrics = glyph_rasterizer.get_glyph(c, self.font_size);

            if x - glyph_metrics.advance.0 <= 0.0 {
                return (false, self.content.byte_slice(byte_index..start_index));
            }

            x -= glyph_metrics.advance.0;
            byte_index -= c.len_utf8();
        }

        // If we haven't returned yet, this is probably the last line
        (false, self.content.byte_slice(start_index..))
    }

    /// Paste content from the system clipboard to the text area at the current position
    pub fn paste(&mut self) {
        let clipboard_contents = self.clipboard_context.get_contents().unwrap();
        self.insert_text(&clipboard_contents);
    }

    pub fn delete(&mut self) {
        let len = self.content.byte_len();
        if len == 0 || self.cursor_position + 1 > self.content.byte_len() {
            return;
        }

        self.content
            .delete(self.cursor_position..self.cursor_position + 1)
    }

    pub fn backspace(&mut self) {
        let len = self.content.byte_len();
        if len == 0 || self.cursor_position == 0 {
            return;
        }

        // Find char boundry one character back
        let mut curr_pos = self.cursor_position;
        loop {
            curr_pos = curr_pos.saturating_sub(1);

            if self.content.is_char_boundary(curr_pos) {
                break;
            }
        }

        self.content
            .delete(self.cursor_position - 1..self.cursor_position);
        self.cursor_position -= 1;
    }

    pub fn insert_text(&mut self, text: &str) {
        self.content.insert(self.cursor_position, text);

        // Needed to handle emojis correctly, as well as regular ascii
        let mut bytes_to_advance = 0;
        for c in text.chars() {
            bytes_to_advance += c.len_utf8();
        }
        dbg!(bytes_to_advance);
        self.cursor_position += bytes_to_advance;
    }

    pub fn scroll(&mut self, scroll: ScrollAmount, glyph_rasterizer: &mut impl GlyphRasterizer) {
        match scroll {
            ScrollAmount::Up { lines } => self.scroll_up(lines, glyph_rasterizer),
            ScrollAmount::Down { lines } => self.scroll_down(lines, glyph_rasterizer),
            ScrollAmount::ToStart => self.scroll_to_start(),
            ScrollAmount::ToEnd => self.scroll_to_end(glyph_rasterizer),
        }
    }

    fn scroll_to_start(&mut self) {
        self.text_start_idx = 0;
        self.cursor_position = 0;
    }

    fn scroll_to_end(&mut self, glyph_rasterizer: &mut impl GlyphRasterizer) {
        let bottom = self.content.byte_len().saturating_sub(1);

        self.text_start_idx = bottom;
        self.cursor_position = bottom;
        self.scroll_up(1, glyph_rasterizer); // so we aren't totally at the bottom and see nothing
    }

    /// Scroll the viewport up 'lines' wrapped lines.
    fn scroll_up(&mut self, lines: usize, glyph_rasterizer: &mut impl GlyphRasterizer) {
        let mut byte_idx = self.text_start_idx;

        for _ in 0..lines {
            let (_, line) = self.layout_line_rev(byte_idx, glyph_rasterizer);
            byte_idx = byte_idx.saturating_sub(line.byte_len());
        }

        self.text_start_idx = byte_idx;
    }

    /// Scroll the viewport down 'lines' wrapped lines.
    fn scroll_down(&mut self, lines: usize, glyph_rasterizer: &mut impl GlyphRasterizer) {
        let mut byte_idx = self.text_start_idx;

        for _ in 0..lines {
            let (has_trailing_newline, line) = self.layout_line(byte_idx, glyph_rasterizer);
            if has_trailing_newline {
                byte_idx += 1;
            }

            byte_idx += line.byte_len();
        }

        self.text_start_idx = byte_idx;
    }
}
