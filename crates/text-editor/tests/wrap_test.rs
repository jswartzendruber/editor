mod helpers;

use crate::helpers::join_rope_slice;
use text_editor::TextEditor;

#[test]
fn layout_single_line_cut_off() {
    let input = "this input is short";
    let editor = TextEditor::new(input, 10);
    let line = editor.layout_line_naive(0);

    assert_eq!(line.to_string(), "this input");
}

#[test]
fn layout_single_line_full() {
    let input = "this";
    let editor = TextEditor::new(input, 10);
    let line = editor.layout_line_naive(0);

    assert_eq!(line.to_string(), "this");
}

#[test]
fn layout_single_line_respects_char_boundry() {
    let input = "興興興興興興興興興興興興興興興興興";
    let editor = TextEditor::new(input, 10);
    let line = editor.layout_line_naive(0);

    assert_eq!(line.to_string(), "興興興興興興興興興興");
}

#[test]
fn layout_single_line_respects_char_boundry_short() {
    let input = "興興興興興興";
    let editor = TextEditor::new(input, 10);
    let line = editor.layout_line_naive(0);

    assert_eq!(line.to_string(), "興興興興興興");
}

#[test]
fn no_wrapping() {
    let input = "this input is short and should not be wrapped.";
    let editor = TextEditor::new(input, 80);
    let lines = editor.layout_lines_naive(80);
    let line = join_rope_slice(&lines);

    let expected = format!("{}\n", input);

    assert_eq!(line.to_string(), expected);
}

#[test]
fn has_newline() {
    let input = "this input is wrapped here\nand this is on a new line.";
    let editor = TextEditor::new(input, 10);
    let lines = editor.layout_lines_naive(80);
    let line = join_rope_slice(&lines);

    let expected = "this input\n is wrappe\nd here\nand this i\ns on a new\n line.\n";

    assert_eq!(line.to_string(), expected);
}

#[test]
fn wrap_at_10() {
    let input = "This text should be wrapped several times.";
    let editor = TextEditor::new(input, 10);
    let lines = editor.layout_lines_naive(10);
    let line = join_rope_slice(&lines);

    let expected = "This text \nshould be \nwrapped se\nveral time\ns.\n";

    assert_eq!(line.to_string(), expected);
}
