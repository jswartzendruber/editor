#![no_main]

use libfuzzer_sys::fuzz_target;
use text_editor::TextEditor;

fuzz_target!(|data: String| {
    let start_idx = 0;

    if data.len() > start_idx {
        let mut editor = TextEditor::new(&data, 10);
        let lines = editor.layout_lines_naive(10);
    }
});
