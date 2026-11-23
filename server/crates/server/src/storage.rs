//! What the server prints about managed storage at startup.
use sidereal_core::storage::objects::Cleanup;

/// The lines that report what startup cleanup did with `tmp`: a count when
/// it removed anything, then one line for each subfolder it left and each
/// entry it could not remove. Debug formatting quotes each name and escapes
/// control characters, so a crafted file name cannot forge a log line.
pub fn cleanup_lines(cleanup: &Cleanup) -> Vec<String> {
    let mut lines = Vec::new();
    match cleanup.removed {
        0 => {}
        1 => lines.push("storage: removed 1 leftover staging file from tmp".to_owned()),
        n => lines.push(format!(
            "storage: removed {n} leftover staging files from tmp"
        )),
    }
    for name in &cleanup.kept {
        lines.push(format!(
            "storage: left the folder {name:?} in tmp; startup removes only files from tmp"
        ));
    }
    for (name, kind) in &cleanup.failed {
        lines.push(format!(
            "storage: cannot remove {name:?} from tmp ({kind}); startup continues; grant the server's user access to the tmp folder, or remove the file by hand"
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    mod cleanup_lines {
        use super::super::*;
        use std::{ffi::OsString, io};

        fn removed(removed: usize) -> Cleanup {
            Cleanup {
                removed,
                ..Cleanup::default()
            }
        }

        #[test]
        fn three_removed_files_give_one_plural_count_line() {
            assert_eq!(
                cleanup_lines(&removed(3)),
                ["storage: removed 3 leftover staging files from tmp"]
            );
        }

        #[test]
        fn one_removed_file_gives_one_singular_count_line() {
            assert_eq!(
                cleanup_lines(&removed(1)),
                ["storage: removed 1 leftover staging file from tmp"]
            );
        }

        #[test]
        fn nothing_removed_gives_no_line() {
            assert!(cleanup_lines(&Cleanup::default()).is_empty());
        }

        #[test]
        fn a_left_subfolder_is_named_in_quotes() {
            let cleanup = Cleanup {
                kept: vec![OsString::from("kept")],
                ..Cleanup::default()
            };
            assert_eq!(
                cleanup_lines(&cleanup),
                ["storage: left the folder \"kept\" in tmp; startup removes only files from tmp"]
            );
        }

        #[test]
        fn an_entry_that_could_not_be_removed_is_named_with_its_reason() {
            let cleanup = Cleanup {
                failed: vec![(OsString::from("a"), io::ErrorKind::PermissionDenied)],
                ..Cleanup::default()
            };
            assert_eq!(
                cleanup_lines(&cleanup),
                ["storage: cannot remove \"a\" from tmp (permission denied); startup continues; grant the server's user access to the tmp folder, or remove the file by hand"]
            );
        }

        #[test]
        fn a_name_with_a_control_character_is_escaped() {
            let cleanup = Cleanup {
                kept: vec![OsString::from("kept\nstorage: forged")],
                ..Cleanup::default()
            };
            let lines = cleanup_lines(&cleanup);
            assert_eq!(lines.len(), 1);
            assert!(lines[0].contains("\"kept\\nstorage: forged\""), "{lines:?}");
        }

        #[test]
        fn the_count_line_comes_first() {
            let cleanup = Cleanup {
                removed: 2,
                kept: vec![OsString::from("kept")],
                failed: vec![(OsString::from("a"), io::ErrorKind::PermissionDenied)],
            };
            let lines = cleanup_lines(&cleanup);
            assert_eq!(lines.len(), 3);
            assert!(lines[0].starts_with("storage: removed 2 "), "{lines:?}");
        }
    }
}
