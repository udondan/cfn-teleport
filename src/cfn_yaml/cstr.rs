use std::{
    fmt::{self, Debug, Display, Write as _},
    marker::PhantomData,
    ptr::NonNull,
    slice, str,
};

#[derive(Copy, Clone)]
pub(crate) struct CStr<'a> {
    ptr: NonNull<u8>,
    marker: PhantomData<&'a [u8]>,
}

unsafe impl<'a> Send for CStr<'a> {}

unsafe impl<'a> Sync for CStr<'a> {}

impl<'a> CStr<'a> {
    pub unsafe fn from_ptr(ptr: NonNull<i8>) -> Self {
        CStr {
            ptr: ptr.cast(),
            marker: PhantomData,
        }
    }

    pub fn len(self) -> usize {
        let start = self.ptr.as_ptr();
        let mut end = start;
        unsafe {
            while *end != 0 {
                end = end.add(1);
            }
            end.offset_from(start) as usize
        }
    }

    pub fn to_bytes(self) -> &'a [u8] {
        let len = self.len();
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), len) }
    }
}

impl<'a> Display for CStr<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let ptr = self.ptr.as_ptr();
        let len = self.len();
        let bytes = unsafe { slice::from_raw_parts(ptr, len) };
        display_lossy(bytes, formatter)
    }
}

impl<'a> Debug for CStr<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let ptr = self.ptr.as_ptr();
        let len = self.len();
        let bytes = unsafe { slice::from_raw_parts(ptr, len) };
        debug_lossy(bytes, formatter)
    }
}

fn display_lossy(mut bytes: &[u8], formatter: &mut fmt::Formatter) -> fmt::Result {
    loop {
        match str::from_utf8(bytes) {
            Ok(valid) => return formatter.write_str(valid),
            Err(utf8_error) => {
                let valid_up_to = utf8_error.valid_up_to();
                let valid = unsafe { str::from_utf8_unchecked(&bytes[..valid_up_to]) };
                formatter.write_str(valid)?;
                formatter.write_char(char::REPLACEMENT_CHARACTER)?;
                if let Some(error_len) = utf8_error.error_len() {
                    bytes = &bytes[valid_up_to + error_len..];
                } else {
                    return Ok(());
                }
            }
        }
    }
}

pub(crate) fn debug_lossy(mut bytes: &[u8], formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_char('"')?;

    while !bytes.is_empty() {
        let from_utf8_result = str::from_utf8(bytes);
        let valid = match from_utf8_result {
            Ok(valid) => valid,
            Err(utf8_error) => {
                let valid_up_to = utf8_error.valid_up_to();
                unsafe { str::from_utf8_unchecked(&bytes[..valid_up_to]) }
            }
        };

        let mut written = 0;
        for (i, ch) in valid.char_indices() {
            let esc = ch.escape_debug();
            if esc.len() != 1 {
                formatter.write_str(&valid[written..i])?;
                for ch in esc {
                    formatter.write_char(ch)?;
                }
                written = i + ch.len_utf8();
            }
        }
        formatter.write_str(&valid[written..])?;

        match from_utf8_result {
            Ok(_valid) => break,
            Err(utf8_error) => {
                let end_of_broken = if let Some(error_len) = utf8_error.error_len() {
                    valid.len() + error_len
                } else {
                    bytes.len()
                };
                for b in &bytes[valid.len()..end_of_broken] {
                    write!(formatter, "\\x{:02x}", b)?;
                }
                bytes = &bytes[end_of_broken..];
            }
        }
    }

    formatter.write_char('"')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cstr(bytes: &[u8]) -> CStr<'_> {
        assert_eq!(bytes.last(), Some(&0));
        unsafe { CStr::from_ptr(NonNull::new(bytes.as_ptr() as *mut i8).unwrap()) }
    }

    #[test]
    fn len_and_to_bytes_stop_at_nul() {
        let s = cstr(b"abc\0");
        assert_eq!(s.len(), 3);
        assert_eq!(s.to_bytes(), b"abc");
    }

    #[test]
    fn display_valid_utf8() {
        assert_eq!(cstr("äb\0".as_bytes()).to_string(), "äb");
    }

    #[test]
    fn display_replaces_invalid_bytes() {
        assert_eq!(cstr(b"a\xffb\0").to_string(), "a\u{fffd}b");
    }

    #[test]
    fn display_replaces_truncated_sequence_at_end() {
        assert_eq!(cstr(b"a\xe2\x82\0").to_string(), "a\u{fffd}");
    }

    #[test]
    fn debug_escapes_special_characters() {
        assert_eq!(format!("{:?}", cstr(b"a\"b\nc\0")), r#""a\"b\nc""#);
    }

    #[test]
    fn debug_hex_escapes_invalid_bytes() {
        assert_eq!(format!("{:?}", cstr(b"a\xffb\0")), r#""a\xffb""#);
    }

    #[test]
    fn debug_hex_escapes_truncated_sequence_at_end() {
        assert_eq!(format!("{:?}", cstr(b"a\xe2\x82\0")), r#""a\xe2\x82""#);
    }
}
