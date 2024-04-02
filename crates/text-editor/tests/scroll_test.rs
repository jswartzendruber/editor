mod helpers;

use crate::helpers::join_rope_slice;
use text_editor::{ScrollAmount, TextEditor};

#[test]
fn scroll_several_times() {
    let input = "this input should be wrapped a few times.";
    let wrap_at = 10;
    let mut editor = TextEditor::new(input, wrap_at);

    let lines_before = editor.layout_lines_naive(80);
    let line_before = join_rope_slice(&lines_before);
    let expected_before = "this input\n should be\n wrapped a\n few times\n.\n";
    assert_eq!(line_before, expected_before);

    editor.scroll(ScrollAmount::Down { lines: 2 });

    let lines_after = editor.layout_lines_naive(80);
    let line_after = join_rope_slice(&lines_after);
    let expected_after = " wrapped a\n few times\n.\n";
    assert_eq!(line_after, expected_after);

    editor.scroll(ScrollAmount::Up { lines: 3 });

    let lines_before = editor.layout_lines_naive(80);
    let line_before = join_rope_slice(&lines_before);
    let expected_before = "this input\n should be\n wrapped a\n few times\n.\n";
    assert_eq!(line_before, expected_before);
}
