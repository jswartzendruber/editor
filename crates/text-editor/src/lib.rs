use crop::{Rope, RopeBuilder, RopeSlice};

#[derive(Debug)]
pub enum ScrollAmount {
    Up { lines: usize },
    Down { lines: usize },
}

#[derive(Debug)]
pub struct TextEditor {
    /// Contains all of the text inside of this text editor.
    content: Rope,

    /// The current position of the cursor in the text rope.
    cursor_position: usize,

    /// The starting index of the text that will be rendered.
    text_start_idx: usize,

    /// The column that text will be wrapped at
    wrap_at: usize,
}

impl TextEditor {
    /// Creates a text editor using the given content, which will wrap
    /// whenever lines exceed 'wrap_at' characters per line.
    pub fn new(content: &str, wrap_at: usize) -> Self {
        let mut builder = RopeBuilder::new();
        builder.append(content);

        let text = builder.build();

        Self {
            content: text,
            cursor_position: 0,
            text_start_idx: 0,
            wrap_at,
        }
    }

    /// Get the current position of the cursor
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Naive word wrap. Considers newline characters, and text past the 'wrap_at' column for wrapping.
    /// Does nothing special for spaces and other characters, they will be included in the output.
    /// # Example
    ///
    /// ```
    /// use text_editor::TextEditor;
    ///
    /// let input = "This text should be wrapped several times.";
    /// let editor = TextEditor::new(input, 10);
    /// let lines = editor.layout_lines_naive(10);
    ///
    /// let expected_lines = &["This text ", "should be ", "wrapped se", "veral time", "s."];
    /// ```
    pub fn layout_lines_naive(&self, max_lines_to_layout: usize) -> Vec<RopeSlice<'_>> {
        if self.content.byte_len() == 0 {
            return vec![];
        }

        let mut start_idx = self.text_start_idx;
        if start_idx >= self.content.byte_len() {
            panic!(
                "{}",
                format!(
                    "index '{}' is out of bounds of content with length '{}'",
                    start_idx,
                    self.content.byte_len()
                )
            )
        }

        let mut lines = vec![];

        // Detect infinite loops when fuzzing.
        // Not that there are any...
        let mut loop_fuel = 10000;

        loop {
            if lines.len() >= max_lines_to_layout || start_idx >= self.content.byte_len() {
                break;
            }

            let line = self.layout_line_naive(start_idx);

            // If the character right after this line is a newline, we need
            // to skip over it before calling layout_line_naive again. Otherwise
            // it would give us a zero length line.
            let last_char_index = start_idx + line.byte_len();
            if last_char_index < self.content.byte_len()
                && self.content.is_char_boundary(last_char_index)
                && self.content.byte(last_char_index) == b'\n'
            {
                start_idx += 1;
            }

            start_idx += line.byte_len();
            lines.push(line);

            loop_fuel -= 1;
            if loop_fuel <= 0 {
                panic!("layout_lines_naive infinite loop");
            }
        }

        lines
    }

    /// Lays out one line from the supplied index into the text buffer.
    /// When we reach the wrap_at limit, or find a newline character,
    /// the slice of this line is returned.
    pub fn layout_line_naive(&self, idx: usize) -> RopeSlice<'_> {
        if idx >= self.content.byte_len() {
            panic!(
                "{}",
                format!(
                    "index '{}' is out of bounds of content with length '{}'",
                    idx,
                    self.content.byte_len()
                )
            )
        }

        let line_start = idx;

        let mut byte_index = idx;
        let mut char_count = 0;

        // Detect infinite loops when fuzzing.
        // Not that there are any...
        let mut loop_fuel = 10000;

        loop {
            // Find the character boundary
            while !self.content.is_char_boundary(byte_index) {
                byte_index += 1;
            }

            // If we are at the end of the string, we need to cap our
            // index at the end of the string and bail
            if byte_index >= self.content.byte_len() {
                byte_index = self.content.byte_len();
                break;
            }

            // See if we have a newline and found the end of the line, or
            // if we are at our wrap limit.
            let c = self.content.byte(byte_index);
            if c == b'\n' || char_count == self.wrap_at {
                break;
            }

            // Include the current character in our set. Update offsets.
            char_count += 1;
            byte_index += 1;

            loop_fuel -= 1;
            if loop_fuel <= 0 {
                panic!("layout_line_naive infinite loop");
            }
        }

        self.content.byte_slice(line_start..byte_index)
    }

    pub fn backspace(&mut self) {
        let len = self.content.byte_len();
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
        }
    }

    /// Scroll the viewport up 'lines' wrapped lines.
    fn scroll_up(&mut self, lines: usize) {
        let mut byte_index = self.text_start_idx;

        // Detect infinite loops when fuzzing.
        // Not that there are any...
        let mut loop_fuel = 10000;

        let mut lines_passed = 0;

        while lines_passed < lines {
            let mut char_count = 0;
            'line_loop: while byte_index > 0 {
                // Find the character boundary
                while (!self.content.is_char_boundary(byte_index) && byte_index > 0)
                    || byte_index >= self.content.byte_len()
                {
                    byte_index -= 1;
                }

                // See if we have a newline and found the end of the line, or
                // if we are at our wrap limit.
                let c = self.content.byte(byte_index);
                while c == b'\n'
                    && byte_index <= self.content.byte_len()
                    && self.content.is_char_boundary(byte_index)
                {
                    byte_index -= 1;
                    break 'line_loop;
                }
                if char_count == self.wrap_at {
                    break;
                }

                // Include the current character in our set. Update offsets.
                char_count += 1;
                if byte_index > 0 {
                    byte_index -= 1;
                }

                loop_fuel -= 1;
                if loop_fuel <= 0 {
                    panic!("layout_line_naive infinite loop");
                }
            }

            lines_passed += 1;
        }

        while byte_index > 0 && self.content.byte(byte_index - 1) != b'\n' {
            byte_index -= 1;
        }

        self.text_start_idx = byte_index;
    }

    /// Scroll the viewport down 'lines' wrapped lines.
    fn scroll_down(&mut self, lines: usize) {
        let mut byte_index = self.text_start_idx;

        // Detect infinite loops when fuzzing.
        // Not that there are any...
        let mut loop_fuel = 10000;

        let mut lines_passed = 0;

        while lines_passed < lines {
            let mut char_count = 0;
            loop {
                // Find the character boundary
                while !self.content.is_char_boundary(byte_index) {
                    byte_index += 1;
                }

                // If we are at the end of the string, we need to cap our
                // index at the end of the string and bail
                if byte_index >= self.content.byte_len() {
                    byte_index = self.content.byte_len();
                    break;
                }

                // See if we have a newline and found the end of the line, or
                // if we are at our wrap limit.
                let c = self.content.byte(byte_index);
                if c == b'\n' || char_count == self.wrap_at {
                    break;
                }

                // Include the current character in our set. Update offsets.
                char_count += 1;
                byte_index += 1;

                loop_fuel -= 1;
                if loop_fuel <= 0 {
                    panic!("layout_line_naive infinite loop");
                }
            }

            // skip past the newline character if there was one
            while byte_index + 1 < self.content.byte_len()
                && self.content.is_char_boundary(byte_index)
                && self.content.byte(byte_index) == b'\n'
            {
                byte_index += 1;
            }

            lines_passed += 1;
        }

        self.text_start_idx = byte_index;
    }
}
