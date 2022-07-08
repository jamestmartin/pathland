let DITHER_BASE = 16;
let BIT_WIDTH = 16;
let DITHER_SIZE = 0x10000;

fn bit_reverse(_x: u32) -> u32 {
    var x = _x;
    var hi = u32(1) << (u32(BIT_WIDTH)-u32(1));
    var lo = u32(1);
    for (var i = u32(0); i < u32(BIT_WIDTH)/u32(2); i = i + u32(1)) {
        let bit_hi = x & hi;
        let bit_lo = x & lo;
        x = x & ~hi & ~lo;
        if (bit_hi > u32(0)) { x = x | lo; }
        if (bit_lo > u32(0)) { x = x | hi; }
        hi = hi >> u32(1);
        lo = lo >> u32(1);
    }
    return x;
}

fn bit_interleave(x: u32, y: u32) -> u32 {
    var mask = u32(1) << (u32(BIT_WIDTH)-u32(1));
    var acc = u32(0);
    for (var i = u32(0); i < u32(BIT_WIDTH); i = i + u32(1)) {
        acc = acc | ((x & mask) << u32(2)*i + u32(1));
        acc = acc | ((y & mask) << u32(2)*i);
        mask = mask >> u32(1);
    }
    return acc;
}

fn bayer(coord: vec2<u32>) -> f32 {
    let magic = bit_reverse(bit_interleave(coord.x ^ coord.y, coord.x));
    return (f32(magic+u32(1)) / (f32(DITHER_SIZE)*f32(DITHER_SIZE))) - 0.5;
}

fn bayer_bias(_pixel: vec2<u32>) -> vec4<f32> {
    let pixel = _pixel % u32(DITHER_SIZE);
    return vec4<f32>(
        bayer(pixel),
        bayer(vec2<u32>(u32(DITHER_SIZE) - pixel.x - u32(1), pixel.y)),
        bayer(vec2<u32>(pixel.x, u32(DITHER_SIZE) - pixel.y - u32(1))),
        bayer(vec2<u32>(u32(DITHER_SIZE) - pixel.x - u32(1), u32(DITHER_SIZE) - pixel.y - u32(1)))
    );
}
