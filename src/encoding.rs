//! Encoding utilities. Available only with the `encoding` feature.

use std::{borrow::Cow, ops::ControlFlow};

use encoding_rs::{Decoder, DecoderResult, Encoding};

use crate::{loader::LoadError, Yaml};

/// The signature of the function to call when using [`YAMLDecodingTrap::Call`].
///
/// The arguments are as follows:
///  * `malformation_length`: The length of the sequence the decoder failed to decode.
///  * `bytes_read_after_malformation`: The number of lookahead bytes the decoder consumed after
///    the malformation.
///  * `input_at_malformation`: What the input buffer is at the malformation.
///    This is the buffer starting at the malformation. The first `malformation_length` bytes are
///    the problematic sequence. The following `bytes_read_after_malformation` are already stored
///    in the decoder and will not be re-fed.
///  * `output`: The output string.
///
/// The function must modify `output` as it feels is best. For instance, one could recreate the
/// behavior of [`YAMLDecodingTrap::Ignore`] with an empty function, [`YAMLDecodingTrap::Replace`]
/// by pushing a `\u{FFFD}` into `output` and [`YAMLDecodingTrap::Strict`] by returning
/// [`ControlFlow::Break`].
///
/// # Returns
/// The function must return [`ControlFlow::Continue`] if decoding may continue or
/// [`ControlFlow::Break`] if decoding must be aborted. An optional error string may be supplied.
pub type YAMLDecodingTrapFn = fn(
    malformation_length: u8,
    bytes_read_after_malformation: u8,
    input_at_malformation: &[u8],
    output: &mut String,
) -> ControlFlow<Cow<'static, str>>;

/// The behavior [`YamlDecoder`] must have when an decoding error occurs.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum YAMLDecodingTrap {
    /// Ignore the offending bytes, remove them from the output.
    Ignore,
    /// Error out.
    Strict,
    /// Replace them with the Unicode REPLACEMENT CHARACTER.
    Replace,
    /// Call the user-supplied function upon decoding malformation.
    Call(YAMLDecodingTrapFn),
}

/// `YamlDecoder` is a `YamlLoader` builder that allows you to supply your own encoding error trap.
/// For example, to read a YAML file while ignoring Unicode decoding errors you can set the
/// `encoding_trap` to `encoding::DecoderTrap::Ignore`.
/// ```rust
/// use saphyr::{YamlDecoder, YAMLDecodingTrap};
///
/// let string = b"---
/// a\xa9: 1
/// b: 2.2
/// c: [1, 2]
/// ";
/// let out = YamlDecoder::read(string as &[u8])
///     .encoding_trap(YAMLDecodingTrap::Ignore)
///     .decode()
///     .unwrap();
/// ```
pub struct YamlDecoder<T: std::io::Read> {
    /// The input stream.
    source: T,
    /// The behavior to adopt when encountering a malformed encoding.
    trap: YAMLDecodingTrap,
}

impl<T: std::io::Read> YamlDecoder<T> {
    /// Create a `YamlDecoder` decoding the given source.
    pub fn read(source: T) -> YamlDecoder<T> {
        YamlDecoder {
            source,
            trap: YAMLDecodingTrap::Strict,
        }
    }

    /// Set the behavior of the decoder when the encoding is invalid.
    pub fn encoding_trap(&mut self, trap: YAMLDecodingTrap) -> &mut Self {
        self.trap = trap;
        self
    }

    /// Run the decode operation with the source and trap the `YamlDecoder` was built with.
    ///
    /// # Errors
    /// Returns `LoadError` when decoding fails.
    pub fn decode(&mut self) -> Result<Vec<Yaml>, LoadError> {
        let mut buffer = Vec::new();
        self.source.read_to_end(&mut buffer)?;

        // Check if the `encoding` library can detect encoding from the BOM, otherwise use
        // `detect_utf16_endianness`.
        let (encoding, _) =
            Encoding::for_bom(&buffer).unwrap_or_else(|| (detect_utf16_endianness(&buffer), 2));
        let mut decoder = encoding.new_decoder();
        let mut output = String::new();

        // Decode the input buffer.
        decode_loop(&buffer, &mut output, &mut decoder, self.trap)?;

        crate::load_from_str(&output).map_err(LoadError::Scan)
    }
}

/// Perform a loop of [`Decoder::decode_to_string`], reallocating `output` if needed.
fn decode_loop(
    input: &[u8],
    output: &mut String,
    decoder: &mut Decoder,
    trap: YAMLDecodingTrap,
) -> Result<(), LoadError> {
    use crate::loader::LoadError;

    output.reserve(input.len());
    let mut total_bytes_read = 0;

    loop {
        match decoder.decode_to_string_without_replacement(&input[total_bytes_read..], output, true)
        {
            // If the input is empty, we processed the whole input.
            (DecoderResult::InputEmpty, _) => break Ok(()),
            // If the output is full, we must reallocate.
            (DecoderResult::OutputFull, bytes_read) => {
                total_bytes_read += bytes_read;
                // The output is already reserved to the size of the input. We slowly resize. Here,
                // we're expecting that 10% of bytes will double in size when converting to UTF-8.
                output.reserve(input.len() / 10);
            }
            (DecoderResult::Malformed(malformed_len, bytes_after_malformed), bytes_read) => {
                total_bytes_read += bytes_read;
                match trap {
                    // Ignore (skip over) malformed character.
                    YAMLDecodingTrap::Ignore => {}
                    // Replace them with the Unicode REPLACEMENT CHARACTER.
                    YAMLDecodingTrap::Replace => {
                        output.push('\u{FFFD}');
                    }
                    // Otherwise error, getting as much context as possible.
                    YAMLDecodingTrap::Strict => {
                        let malformed_len = malformed_len as usize;
                        let bytes_after_malformed = bytes_after_malformed as usize;
                        let byte_idx = total_bytes_read - (malformed_len + bytes_after_malformed);
                        let malformed_sequence = &input[byte_idx..byte_idx + malformed_len];

                        break Err(LoadError::Decode(Cow::Owned(format!(
                            "Invalid character sequence at {byte_idx}: {malformed_sequence:?}",
                        ))));
                    }
                    YAMLDecodingTrap::Call(callback) => {
                        let byte_idx =
                            total_bytes_read - ((malformed_len + bytes_after_malformed) as usize);
                        let malformed_sequence =
                            &input[byte_idx..byte_idx + malformed_len as usize];
                        if let ControlFlow::Break(error) = callback(
                            malformed_len,
                            bytes_after_malformed,
                            &input[byte_idx..],
                            output,
                        ) {
                            if error.is_empty() {
                                break Err(LoadError::Decode(Cow::Owned(format!(
                                    "Invalid character sequence at {byte_idx}: {malformed_sequence:?}",
                                ))));
                            }
                            break Err(LoadError::Decode(error));
                        }
                    }
                }
            }
        }
    }
}

/// The encoding crate knows how to tell apart UTF-8 from UTF-16LE and utf-16BE, when the
/// bytestream starts with BOM codepoint.
/// However, it doesn't even attempt to guess the UTF-16 endianness of the input bytestream since
/// in the general case the bytestream could start with a codepoint that uses both bytes.
///
/// The YAML-1.2 spec mandates that the first character of a YAML document is an ASCII character.
/// This allows the encoding to be deduced by the pattern of null (#x00) characters.
//
/// See spec at <https://yaml.org/spec/1.2/spec.html#id2771184>
fn detect_utf16_endianness(b: &[u8]) -> &'static Encoding {
    if b.len() > 1 && (b[0] != b[1]) {
        if b[0] == 0 {
            return encoding_rs::UTF_16BE;
        } else if b[1] == 0 {
            return encoding_rs::UTF_16LE;
        }
    }
    encoding_rs::UTF_8
}

#[cfg(test)]
mod test {
    use super::{YAMLDecodingTrap, Yaml, YamlDecoder};

    #[test]
    fn test_read_bom() {
        let s = b"\xef\xbb\xbf---
a: 1
b: 2.2
c: [1, 2]
";
        let out = YamlDecoder::read(s as &[u8]).decode().unwrap();
        let doc = &out[0];
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64).abs() <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_read_utf16le() {
        let s = b"\xff\xfe-\x00-\x00-\x00
\x00a\x00:\x00 \x001\x00
\x00b\x00:\x00 \x002\x00.\x002\x00
\x00c\x00:\x00 \x00[\x001\x00,\x00 \x002\x00]\x00
\x00";
        let out = YamlDecoder::read(s as &[u8]).decode().unwrap();
        let doc = &out[0];
        println!("GOT: {doc:?}");
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64) <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_read_utf16be() {
        let s = b"\xfe\xff\x00-\x00-\x00-\x00
\x00a\x00:\x00 \x001\x00
\x00b\x00:\x00 \x002\x00.\x002\x00
\x00c\x00:\x00 \x00[\x001\x00,\x00 \x002\x00]\x00
";
        let out = YamlDecoder::read(s as &[u8]).decode().unwrap();
        let doc = &out[0];
        println!("GOT: {doc:?}");
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64).abs() <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_read_utf16le_nobom() {
        let s = b"-\x00-\x00-\x00
\x00a\x00:\x00 \x001\x00
\x00b\x00:\x00 \x002\x00.\x002\x00
\x00c\x00:\x00 \x00[\x001\x00,\x00 \x002\x00]\x00
\x00";
        let out = YamlDecoder::read(s as &[u8]).decode().unwrap();
        let doc = &out[0];
        println!("GOT: {doc:?}");
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64).abs() <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_read_trap() {
        let s = b"---
a\xa9: 1
b: 2.2
c: [1, 2]
";
        let out = YamlDecoder::read(s as &[u8])
            .encoding_trap(YAMLDecodingTrap::Ignore)
            .decode()
            .unwrap();
        let doc = &out[0];
        println!("GOT: {doc:?}");
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64).abs() <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_or() {
        assert_eq!(Yaml::Null.or(Yaml::Integer(3)), Yaml::Integer(3));
        assert_eq!(Yaml::Integer(3).or(Yaml::Integer(7)), Yaml::Integer(3));
    }
}
