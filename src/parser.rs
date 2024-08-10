use std::{
    convert::TryInto,
    fmt::{self, Debug, Formatter},
    io::{Cursor, Error, ErrorKind, Read, Result as IoResult, Seek, SeekFrom},
    mem::size_of,
};

#[derive(Debug, Clone, Eq, PartialEq)]
struct Toc {
    toctype: u32,
    subtype: u32,
    pos: u32,
}

/// A struct representing an image.
/// Pixels are in ARGB format, with each byte representing a single channel.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Image {
    /// The nominal size of the image.
    pub size: u32,

    /// The actual width of the image. Doesn't need to match `size`.
    pub width: u32,

    /// The actual height of the image. Doesn't need to match `size`.
    pub height: u32,

    /// The X coordinate of the hotspot pixel (the pixel where the tip of the arrow is situated)
    pub xhot: u32,

    /// The Y coordinate of the hotspot pixel (the pixel where the tip of the arrow is situated)
    pub yhot: u32,

    /// The amount of time (in milliseconds) that this image should be shown for, before switching to the next.
    pub delay: u32,

    /// A slice containing the pixels' bytes, in RGBA format (or, in the order of the file).
    pub pixels_rgba: Vec<u8>,

    /// A slice containing the pixels' bytes, in ARGB format.
    pub pixels_argb: Vec<u8>,
}

impl std::fmt::Display for Image {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("size", &self.size)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("xhot", &self.xhot)
            .field("yhot", &self.yhot)
            .field("delay", &self.delay)
            .field("pixels", &"/* omitted */")
            .finish()
    }
}

fn parse_header(i: &mut impl Read) -> IoResult<u32> {
    i.tag(b"Xcur")?;
    i.u32_le()?;
    i.u32_le()?;
    let ntoc = i.u32_le()?;

    Ok(ntoc)
}

fn parse_toc(i: &mut impl Read) -> IoResult<Toc> {
    let toctype = i.u32_le()?; // Type
    let subtype = i.u32_le()?; // Subtype
    let pos = i.u32_le()?; // Position

    Ok(Toc {
        toctype,
        subtype,
        pos,
    })
}

fn parse_img(i: &mut impl Read) -> IoResult<Image> {
    i.tag(&[0x24, 0x00, 0x00, 0x00])?; // Header size
    i.tag(&[0x02, 0x00, 0xfd, 0xff])?; // Type
    let size = i.u32_le()?;
    i.tag(&[0x01, 0x00, 0x00, 0x00])?; // Image version (1)
    let width = i.u32_le()?;
    let height = i.u32_le()?;
    let xhot = i.u32_le()?;
    let yhot = i.u32_le()?;
    let delay = i.u32_le()?;

    // Check image is well-formed. Taken from https://gitlab.freedesktop.org/xorg/lib/libxcursor/-/blob/09617bcc9a0f1b5072212da5f8fede92ab85d157/src/file.c#L456-463
    if width > 0x7fff || height > 0x7fff {
        return Err(Error::new(ErrorKind::Other, "Image too large"));
    }
    if width == 0 || height == 0 {
        return Err(Error::new(
            ErrorKind::Other,
            "Image with zero width or height",
        ));
    }
    if xhot > width || yhot > height {
        return Err(Error::new(ErrorKind::Other, "Hotspot outside image"));
    }

    let img_length: usize = (4 * width * height) as usize;
    let pixels_rgba = i.take_bytes(img_length)?;
    let pixels_argb = rgba_to_argb(&pixels_rgba);

    Ok(Image {
        size,
        width,
        height,
        xhot,
        yhot,
        delay,
        pixels_argb,
        pixels_rgba,
    })
}

/// Converts a RGBA slice into an ARGB vec
///
/// Note that, if the input length is not
/// a multiple of 4, the extra elements are ignored.
fn rgba_to_argb(i: &[u8]) -> Vec<u8> {
    let mut res = Vec::with_capacity(i.len());

    for rgba in i.chunks(4) {
        if rgba.len() < 4 {
            break;
        }

        res.push(rgba[3]);
        res.push(rgba[0]);
        res.push(rgba[1]);
        res.push(rgba[2]);
    }

    res
}

/// Parse an XCursor file into its images.
pub fn parse_xcursor(content: &[u8]) -> Option<Vec<Image>> {
    parse_xcursor_stream(&mut Cursor::new(content)).ok()
}

/// Parse an XCursor file into its images.
pub fn parse_xcursor_stream<R: Read + Seek>(input: &mut R) -> IoResult<Vec<Image>> {
    let ntoc = parse_header(input)?;

    let mut img_indices = Vec::new();
    for _ in 0..ntoc {
        let toc = parse_toc(input)?;

        if toc.toctype == 0xfffd_0002 {
            img_indices.push(toc.pos);
        }
    }

    let mut imgs = Vec::with_capacity(ntoc as usize);
    for index in img_indices {
        input.seek(SeekFrom::Start(index.into()))?;
        imgs.push(parse_img(input)?);
    }

    Ok(imgs)
}

trait StreamExt {
    /// Parse a series of bytes, returning `None` if it doesn't exist.
    fn tag(&mut self, tag: &[u8]) -> IoResult<()>;

    /// Take a slice of bytes.
    fn take_bytes(&mut self, len: usize) -> IoResult<Vec<u8>>;

    /// Parse a 32-bit little endian number.
    fn u32_le(&mut self) -> IoResult<u32>;
}

impl<R: Read> StreamExt for R {
    fn tag(&mut self, tag: &[u8]) -> IoResult<()> {
        let mut data = vec![0; tag.len()];
        self.read_exact(&mut data)?;
        if data != *tag {
            Err(Error::new(ErrorKind::Other, "Tag mismatch"))
        } else {
            Ok(())
        }
    }

    fn take_bytes(&mut self, len: usize) -> IoResult<Vec<u8>> {
        let mut data = vec![0; len];
        self.read_exact(&mut data)?;
        Ok(data)
    }

    fn u32_le(&mut self) -> IoResult<u32> {
        self.take_bytes(size_of::<u32>())
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_header, parse_toc, rgba_to_argb, Toc};
    use std::io::Cursor;

    // A sample (and simple) XCursor file generated with xcursorgen.
    // Contains a single 4x4 image.
    const FILE_CONTENTS: [u8; 128] = [
        0x58, 0x63, 0x75, 0x72, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x02, 0x00, 0xFD, 0xFF, 0x04, 0x00, 0x00, 0x00, 0x1C, 0x00, 0x00, 0x00, 0x24, 0x00,
        0x00, 0x00, 0x02, 0x00, 0xFD, 0xFF, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04,
        0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00,
        0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00,
        0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00,
        0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80,
        0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80,
    ];

    #[test]
    fn test_parse_header() {
        let mut cursor = Cursor::new(&FILE_CONTENTS);
        assert_eq!(parse_header(&mut cursor).unwrap(), 1);
        assert_eq!(cursor.position(), 16);
    }

    #[test]
    fn test_parse_toc() {
        let toc = Toc {
            toctype: 0xfffd0002,
            subtype: 4,
            pos: 0x1c,
        };
        let mut cursor = Cursor::new(&FILE_CONTENTS[16..]);
        assert_eq!(parse_toc(&mut cursor).unwrap(), toc);
        assert_eq!(cursor.position(), 28 - 16);
    }

    #[test]
    fn test_rgba_to_argb() {
        let initial: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

        assert_eq!(rgba_to_argb(&initial), [3u8, 0, 1, 2, 7, 4, 5, 6])
    }

    #[test]
    fn test_rgba_to_argb_extra_items() {
        let initial: [u8; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];

        assert_eq!(rgba_to_argb(&initial), &[3u8, 0, 1, 2, 7, 4, 5, 6]);
    }

    #[test]
    fn test_rgba_to_argb_no_items() {
        let initial: &[u8] = &[];

        assert_eq!(initial, &rgba_to_argb(initial)[..]);
    }
}
