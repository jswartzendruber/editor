use crop::{Rope, RopeBuilder};

#[derive(Debug)]
pub enum ScrollAmount {
    Up { lines: usize },
    Down { lines: usize },
}

#[derive(Debug)]
pub struct TextEditor {
    /// Contains all of the text inside of this text editor.
    pub content: Rope,

    /// The current position of the cursor in the text rope.
    pub cursor_position: usize,

    /// This is the line number of the text rope that will be displayed. Anything
    /// above this number will not be rendered.
    pub text_start_line: usize,
}

impl TextEditor {
    pub fn new(content: &str) -> Self {
        let mut builder = RopeBuilder::new();
        builder.append(content);

        let text = builder.build();

        Self {
            content: text,
            cursor_position: 0,
            text_start_line: 0,
        }
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
            ScrollAmount::Up { lines } => {
                if self.text_start_line.saturating_sub(lines) > 0 {
                    self.text_start_line -= lines;
                }
            }
            ScrollAmount::Down { lines } => {
                if self.text_start_line + lines < self.content.line_len() {
                    self.text_start_line += lines;
                }
            }
        }
    }
}

#[test]
fn it_works() {
    assert_eq!(4, 4);
}
