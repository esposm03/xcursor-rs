use std::{fmt, fmt::{Debug, Formatter}};

use nom::bytes::complete as bytes;
use nom::number::complete as number;
use nom::IResult;

#[derive(Debug, Clone, Eq, PartialEq)]
struct TOC {
    toctype: u32,
    subtype: u32,
    pos: u32,
}

/// A struct representing an image.
/// Pixels are in ARGB format, with each byte representing a single channel.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Image<'a> {
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

	/// A slice containing the pixels' bytes.
	pixels: &'a [u8],
}

impl std::fmt::Display for Image<'_> {
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

fn parse_header(i: &[u8]) -> IResult<&[u8], u32> {
    let (i, _) = bytes::tag("Xcur")(i)?;
	let (i, _) = number::le_u32(i)?;
	let (i, _) = number::le_u32(i)?;
	let (i, ntoc) = number::le_u32(i)?;

	Ok((i, ntoc))
}

fn parse_toc(i: &[u8]) -> IResult<&[u8], TOC> {
	let (i, toctype) = number::le_u32(i)?; // Type
	let (i, subtype) = number::le_u32(i)?; // Subtype
	let (i, pos) = number::le_u32(i)?; // Position

	Ok((i, TOC {
		toctype,
		subtype,
		pos,
	}))
}

fn parse_img(i: &[u8]) -> IResult<&[u8], Image> {
	let (i, _) = bytes::tag([0x24, 0x00, 0x00, 0x00])(i)?; // Header size
	let (i, _) = bytes::tag([0x02, 0x00, 0xfd, 0xff])(i)?; // Type
	let (i, size) = number::le_u32(i)?;
	let (i, _) = bytes::tag([0x01, 0x00, 0x00, 0x00])(i)?; // Image version (1)
	let (i, width) = number::le_u32(i)?;
	let (i, height) = number::le_u32(i)?;
	let (i, xhot) = number::le_u32(i)?;
	let (i, yhot) = number::le_u32(i)?;
	let (i, delay) = number::le_u32(i)?;

	let img_length: usize = (4 * width * height) as usize;
	let (i, pixels) = bytes::take(img_length)(i)?;

	Ok((i, Image {
		size,
		width,
		height,
		xhot,
		yhot,
		delay,
		pixels,
	}))
}

pub fn parse_xcursor(content: &[u8]) -> Option<Vec<Image>> {
	let (mut i, ntoc) = parse_header(content).ok()?;
	let mut imgs = Vec::with_capacity(ntoc as usize);

	for _ in 0..ntoc {
		let (j, toc) = parse_toc(i).ok()?;
		i = j;

		if toc.toctype == 0xfffd0002 {
			let index = toc.pos as usize..;
			println!("{:x?}", index);
			let (_, img) = parse_img(&content[index]).ok()?;
			imgs.push(img);
		}
	}

	Some(imgs)
}

#[cfg(test)]
mod tests {
	use super::{
		TOC,
		Image,
		parse_header,
		parse_toc,
		parse_img,
	};

	// A sample (and simple) XCursor file generated with xcursorgen.
	// Contains a single 4x4 image.
    const FILE_CONTENTS: [u8; 128] = [
        0x58, 0x63, 0x75, 0x72, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x02, 0x00, 0xFD, 0xFF, 0x04, 0x00, 0x00, 0x00, 0x1C, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00,
        0x02, 0x00, 0xFD, 0xFF, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
        0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80,
        0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80,
        0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80,
        0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80,
    ];

	#[test]
	fn test_parse_header() {
		assert_eq!(
			parse_header(&FILE_CONTENTS).unwrap(),
			(&FILE_CONTENTS[16..], 1)
		)
	}

	#[test]
	fn test_parse_toc() {
		let toc = TOC { toctype: 0xfffd0002, subtype: 4, pos: 0x1c };
		assert_eq!(
			parse_toc(&FILE_CONTENTS[16..]).unwrap(),
			(&FILE_CONTENTS[28..], toc)
		)
	}

	#[test]
	fn test_parse_img() {
		let mut pixels = Vec::new();
		pixels.extend_from_slice(&FILE_CONTENTS[0x1C..]);
		let img = Image {
			delay: 1,
			width: 4,
			height: 4,
			size: 4,
			xhot: 1,
			yhot: 1,
			pixels: &pixels,
		};
		let parsed_img = 

		assert_eq!(
			(&pixels[pixels.len()..], img),
			parse_img(&pixels).unwrap()
		);
	}
}
