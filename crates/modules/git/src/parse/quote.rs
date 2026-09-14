/// Git's C-style quoting undone. `core.quotePath` writes a non-ASCII byte as `\ooo`,
/// so decoding runs over bytes and UTF-8 comes last.
pub fn unquote_path(s: &str) -> String {
    let s = s.trim();
    if !(s.len() >= 2 && s.starts_with('"') && s.ends_with('"')) {
        return s.to_string();
    }
    let inner = &s.as_bytes()[1..s.len() - 1];
    let mut out: Vec<u8> = Vec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        if inner[i] != b'\\' {
            out.push(inner[i]);
            i += 1;
            continue;
        }
        i += 1;
        let Some(&esc) = inner.get(i) else { break };
        i += 1;
        match esc {
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'r' => out.push(b'\r'),
            b'0'..=b'7' => match octal_byte(esc, inner.get(i), inner.get(i + 1)) {
                Some(byte) => {
                    out.push(byte);
                    i += 2;
                }
                None => out.push(esc),
            },
            other => out.push(other),
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Three octal digits as one byte; `None` when the escape is not three digits.
fn octal_byte(first: u8, second: Option<&u8>, third: Option<&u8>) -> Option<u8> {
    let digit = |d: Option<&u8>| {
        d.copied()
            .filter(|d| (b'0'..=b'7').contains(d))
            .map(|d| u32::from(d - b'0'))
    };
    let value = digit(Some(&first))? * 64 + digit(second)? * 8 + digit(third)?;
    u8::try_from(value).ok()
}
