#![no_main]

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use text_editor::TextEditor;

fuzz_target!(|data: &[u8]| {
    let start_idx = 0;

    let mut u = libfuzzer_sys::arbitrary::Unstructured::new(data);
    let s = String::arbitrary(&mut u).unwrap();
    let sn = u8::arbitrary(&mut u).unwrap();
    let su = u8::arbitrary(&mut u).unwrap();
    let sd = u8::arbitrary(&mut u).unwrap();
    let so = u8::arbitrary(&mut u).unwrap();
    let r = u8::arbitrary(&mut u).unwrap();

    if s.len() > start_idx {
        let mut editor = TextEditor::new(&s, 10);

        let mut count = 0;
        for x in 0..sn {
            if count % 2 == 0 {
                editor.scroll(text_editor::ScrollAmount::Down {
                    lines: so as usize,
                });
            } else {
                editor.scroll(text_editor::ScrollAmount::Up {
                    lines: so as usize,
                });
            }

            editor.scroll(text_editor::ScrollAmount::Down {
                lines: sd as usize,
            });
            editor.scroll(text_editor::ScrollAmount::Up {
                lines: su as usize,
            });
            count += r as usize + x as usize;
        }
    }
});
