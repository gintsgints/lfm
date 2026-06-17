use crate::update::detect_text;

/// Plain UTF-8 bytes are accepted and returned verbatim.
#[test]
fn accepts_utf8_text() {
    assert_eq!(
        detect_text("hello\nworld".as_bytes().to_vec()),
        Ok("hello\nworld".to_owned())
    );
}

/// An embedded NUL byte marks the content as binary.
#[test]
fn rejects_nul_byte() {
    assert!(detect_text(vec![b'a', 0, b'b']).is_err());
}

/// Invalid UTF-8 (a lone continuation byte) is rejected as binary.
#[test]
fn rejects_invalid_utf8() {
    assert!(detect_text(vec![0xff, 0xfe]).is_err());
}

/// Empty input is valid, empty text.
#[test]
fn accepts_empty() {
    assert_eq!(detect_text(Vec::new()), Ok(String::new()));
}
