use image::*;

/// Generate Bayer matrix texture for ordered dithering.

/// TODO: better/alternative noise-based matrix to avoid grid pattern?

const DITHER_SIZE: u8 = 1 << 7;



/// Bitwise interleave two integers of length BIT_WIDTH into a single
/// 2*BIT_WIDTH integer.
///
/// example interleave:
///
///      x = 0 1 0 0 1
///      y =  1 0 0 1 1
///         ----------
///      r = 0110000111
///
/// actually also reverses bits, but I want that anyway.
/// (don't try to re-use this function!)
fn bit_interleave(mut x: u8, mut y: u8) -> u16 {
    let mut acc: u16 = 0;
    for _ in 0..8 {
        acc <<= 1;
        acc |= (x & 1) as u16;
        x >>= 1;
        acc <<= 1;
        acc |= (y & 1) as u16;
        y >>= 1;
    }
    acc
}

fn bayer(x: u8, y: u8) -> f32 {
    // Magic bitwise formula from Wikipedia produces values from 0 to 2^16-1.
    // FIXME: slight vertical lines when displaying dither texture
    let magic = bit_interleave(x ^ y, x);
    (magic as f32 + 1.) / (u8::MAX as u16 * u8::MAX as u16) as f32
}

fn bayer_bias(x: u8, y: u8) -> [f32; 4] {
    [
        // TODO: smarter way to re-tile color channels? if this isn't good enough.
        bayer(x, y),
        bayer(DITHER_SIZE.overflowing_sub(x).0, y),
        bayer(x, DITHER_SIZE.overflowing_sub(y).0),
        bayer(DITHER_SIZE.overflowing_sub(x).0, DITHER_SIZE.overflowing_sub(y).0)
    ]
}

pub fn bayer_texture() -> Rgba32FImage {
    ImageBuffer::from_fn(u8::MAX as u32, u8::MAX as u32, |x, y| Rgba::from(bayer_bias(x as u8, y as u8)))
}
