struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    return out;
}

struct Uniforms {
    dimensions: vec2<f32>,
    field_of_view: f32,
}

@group(1)
@binding(0)
var<uniform> uniforms: Uniforms;

let PI: f32 = 3.14159265358979323846264338327950288; // 3.14159274

struct Ray {
    pos: vec3<f32>, // POSition (aka the origin)
    dir: vec3<f32>, // DIRection (normalized)
}

///
/// Convert from pixel coordinates to window-independent square coordinates.
///
/// Input coordinates:
///   x: from 0 (left) to dimensions.x (right)
///   y: from 0 (bottom) to dimensions.y (top)
///
/// Output coordinates:
///   x: from -1 (left) to 1 (right)
///   y: from -1 (down) to 1 (up)
///
/// The output coordinates are square and independent of the
/// window's dimensions and aspect ratio. Some of the image
/// will be cropped if the window's aspect ratio is not square.
fn pixel_to_square(pixel: vec2<f32>) -> vec2<f32> {
    let square = ((pixel / uniforms.dimensions) - 0.5) * 2.0;

    // Scale the window's smaller aspect ratio to make the coordinates square.
    // For example, a 16:9 window will have an x coordinate from -1 to 1 and
    // a y coordinate from -9/16ths to 9/16ths. The rest of the image lying outside
    // of that range will be cropped out.
    if (uniforms.dimensions.x > uniforms.dimensions.y) {
        return vec2<f32>(square.x, square.y * uniforms.dimensions.y / uniforms.dimensions.x);
    } else {
        return vec2<f32>(square.x * uniforms.dimensions.x / uniforms.dimensions.y, square.y);
    }
}

/// Project a coordinate on the unit circle onto the unit hemisphere.
/// This is used for curvilinear perspective.
///
/// Coordinates:
///     x: from -1 (90 degrees left) to 1 (90 degrees right)
///     y: from -1 (90 degrees down) to 1 (90 degrees up)
///
/// TODO: add support for the usual, non-curvilinear perspective projection
/// (and possibly other projections, just for fun?)
fn project(coord_: vec2<f32>) -> vec3<f32> {
    var coord = coord_;
    // This projection only supports coordinates within the unit circle
    // and only projects into the unit hemisphere. Ideally we'd want
    // some sort of extension which takes points outside the unit circle
    // and projects them somewhere behind you (with the point at infinity
    // being directly behind you), but I haven't come up with any reasonable
    // extension of this perspective system which behaves in that manner.
    //
    // What we can do instead is *tile* the projection so that adjacent projections
    // are a mirrored projection of the unit hemisphere *behind* you.
    // This is a logical extension because the projection becomes continuous
    // along the x and y axis (you're just looking around in perfect circles),
    // and it allows you to view the entire space. The main problem to this approach
    // is that all of the space between the tiled circles is still undefined,
    // but this is still the best solution which I'm aware of.

    var dir: f32 = 1.; // the sign of the direction we're facing: 1 forward, -1 backward.
    // Tile coordinates:
    //     (0-2, 0-2): forward
    //     (2-4, 0-2): backward, left/right mirrored
    //     (0-2, 2-4): backward, up/down mirrored
    //     (2-4, 2-4): forward, left/right and up/down mirrored
    // FIXME: Use modulus which handles negatives properly so I don't have to arbitrarily add 8.
    coord = (coord + 1. + 8.) % 4.;
    // mirror/reverse and map back into 0 to 2 range
    if (coord.x > 2.) {
        coord.x = 4. - coord.x;
        dir = -dir;
    }
    if (coord.y > 2.) {
        coord.y = 4. - coord.y;
        dir = -dir;
    }
    // map back into -1 to 1 range
    coord = coord - 1.;

    // Avoid NaN because implementations are allowed to assume it won't occur.
    let preZ = 1. - coord.x*coord.x - coord.y*coord.y;

    // We can "define" the remaining undefined region of the screen
    // by clamping it to the nearest unit circle. This is sometimes
    // better than nothing, though it can also be a lot worse because
    // we still have to actually *render* all of those pixels.

    // TODO: Add an option to allow stretching into a square instead of clamping?
    // I imagine things could get pretty badly warped, but maybe it could be useful?

    // TODO: Is this clamping behavior correct? It doesn't look like it actually is, tbh.
    if (preZ < 0.) {
        return vec3<f32>(normalize(coord), 0.);
    }
    return normalize(vec3<f32>(coord, dir*sqrt(preZ)));
}

/// After converting pixel coordinates to screen coordinates, we still have a problem:
/// screen coordinates are 2d, but our world is 3d! The camera assigns each screen
/// coordinate to a ray in 3d space, indicating the position and angle which
/// we will be receiving light from.
fn camera_project(square: vec2<f32>) -> Ray {
    // Our coordinates already range from -1 to 1, corresponding with the
    // edges of the window, but we want the edges of the window to correspond
    // with the angle of the FOV instead.
    let circle = square * uniforms.field_of_view / PI;
    let sphere = project(circle);
    return Ray(vec3<f32>(0.), sphere);
}

@group(0)
@binding(0)
var dither_texture: texture_2d<f32>;

/// Apply ordered dithering, which reduces color banding and produces the appearance
/// of more colors when in a limited color space (e.g. dark colors with a typical
/// 8-bit sRGB monitor).
// FIXME: document, don't hardcode width/bit depth
fn dither(pixel: vec2<u32>, color: vec4<f32>) -> vec4<f32> {
    // FIXME: issues with bars at edge caused by bad modulus? (should be %256 but pixel rounding incorrect?)
    let bias = textureLoad(dither_texture, vec2<i32>(i32(pixel.x % u32(255)), i32(pixel.y % u32(255))), 0) - 0.5;
    // FIXME: hack to avoid srgb issues
    return color + (bias / 256.);
}

////
//// AUTHOR: Sam Hocevar (http://lolengine.net/blog/2013/07/27/rgb-to-hsv-in-glsl)
////
fn rgb2hsv(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4<f32>(c.bg, K.wz), vec4<f32>(c.gb, K.xy), step(c.b, c.g));
    let q = mix(vec4<f32>(p.xyw, c.r), vec4<f32>(c.r, p.yzx), step(p.x, c.r));

    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3<f32>(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

/// Given a color which clips outside the color space (some channel is >1.0),
/// reduce the brightness (without affecting hue or saturation) until it no
/// longer clips. (The default behavior without doing this is just clipping,
/// which affects the saturation of the color dramatically, often turning colors
/// into 100% white pixels.)
fn clamp_value(_color: vec3<f32>) -> vec3<f32> {
    // TODO: Adjust value directly, without going through HSV conversion.
    var color = rgb2hsv(_color.rgb);
    color.z = min(color.z, 1.); // clamp value (brightness) from 0 to 1, preserving saturation and chroma
    return hsv2rgb(color);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ray = camera_project(pixel_to_square(in.position.xy));
    var color = ray.dir / 2.0 + 0.5;

    // TODO: Separate postprocessing pass.

    // It is possible for this renderer to emit colors brighter than 1.0,
    // for example if you use very bright or many light sources. These colors will be
    // displayed incorrectly, appearing desaturated and having their brightness
    // clamped to whatever color output is supported.
    //
    // This is common in particular if you have very bright lights in a scene,
    // which is sometimes necessary for objects to be clearly visible. The result
    // will be you seeing flashes of over-bright white pixels where you should
    // see color. One way to mitigate this is by increasing the number of samples per
    // pixel; the average brightness per pixel is generally less than 1.0 when averaged
    // out with the (more common) black pixels when no light source is encountered.
    //
    // Another mitigation approach is to do color correction, where instead of
    // trying to preserve the brightness by clamping the RGB values and losing saturation,
    // you try to preserve the saturation by scaling down the brightness until the
    // full saturation of the colors is visible (or at least part of it).
    color = clamp_value(color);

    // Dithering after sRGB conversion is slightly worse because the bayer matrix
    // is linear whereas sRGB is non-linear, but if you do it *before* conversion,
    // then adjusted colors won't be *quite* close enough to nearest_color that they
    // should be closest to, which has the potential to create nasty artifacts.
    //
    // FIXME: This shader uses linear color space.
    return dither(
       vec2<u32>(u32(in.position.x), u32(in.position.y)),
       vec4<f32>(color, 1.0)
    );
}
