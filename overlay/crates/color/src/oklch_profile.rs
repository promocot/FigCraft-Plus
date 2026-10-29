//! OKLCH (Björn Ottosson's OKLab in polar form), the perceptual model the colour picker offers
//! besides hex, RGB, HSL and HSB: lightness 0..1, chroma 0..~0.4 and hue in degrees.
//!
//! A document's colour values are in its profile (sRGB or Display P3), so conversions take one.
//! A colour outside the profile's gamut is brought in the CSS Color 4 way: chroma is reduced (same
//! lightness and hue) until the clipped colour is within a just-noticeable difference.

use crate::{Color, ColorProfile};

/// An OKLCH colour: `l` 0..1, `c` 0.. (sRGB tops out near 0.37, Display P3 near 0.46), `h`
/// degrees 0..360.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

/// What 100% chroma means in CSS (`oklch(L 100% H)` is chroma 0.4).
pub const OKLCH_FULL_CHROMA: f64 = 0.4;

type Mat3 = [[f64; 3]; 3];

fn mul(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

// Linear sRGB <-> LMS <-> OKLab (Ottosson's published matrices).
const LMS_FROM_SRGB: Mat3 = [
    [0.4122214708, 0.5363325363, 0.0514459929],
    [0.2119034982, 0.6806995451, 0.1073969566],
    [0.0883024619, 0.2817188376, 0.6299787005],
];
const LAB_FROM_LMS: Mat3 = [
    [0.2104542553, 0.7936177850, -0.0040720468],
    [1.9779984951, -2.4285922050, 0.4505937099],
    [0.0259040371, 0.7827717662, -0.8086757660],
];
const LMS_FROM_LAB: Mat3 = [
    [1.0, 0.3963377774, 0.2158037573],
    [1.0, -0.1055613458, -0.0638541728],
    [1.0, -0.0894841775, -1.2914855480],
];
const SRGB_FROM_LMS: Mat3 = [
    [4.0767416621, -3.3077115913, 0.2309699292],
    [-1.2684380046, 2.6097574011, -0.3413193965],
    [-0.0041960863, -0.7034186147, 1.7076147010],
];
// Linear sRGB <-> linear Display P3 (both D65).
const P3_FROM_SRGB: Mat3 = [
    [0.8224621209, 0.1775378791, 0.0],
    [0.0331941989, 0.9668058011, 0.0],
    [0.0170826307, 0.0723974407, 0.9105199286],
];
const SRGB_FROM_P3: Mat3 = [
    [1.2249401763, -0.2249401763, 0.0],
    [-0.0420569547, 1.0420569547, 0.0],
    [-0.0196375546, -0.0786360456, 1.0982736002],
];

/// The sRGB transfer curve (Display P3 shares it), sign-preserving.
fn encode(v: f64) -> f64 {
    let a = v.abs();
    let e = if a <= 0.0031308 { 12.92 * a } else { 1.055 * a.powf(1.0 / 2.4) - 0.055 };
    e.copysign(v)
}

fn decode(v: f64) -> f64 {
    let a = v.abs();
    let d = if a <= 0.04045 { a / 12.92 } else { ((a + 0.055) / 1.055).powf(2.4) };
    d.copysign(v)
}

fn finite(v: f64, d: f64) -> f64 {
    if v.is_finite() { v } else { d }
}

impl Oklch {
    /// Clamped to its ranges (lightness 0..1, chroma 0..0.5, hue wrapped to 0..360).
    pub fn new(l: f64, c: f64, h: f64) -> Oklch {
        Oklch { l: finite(l, 0.0).clamp(0.0, 1.0), c: finite(c, 0.0).clamp(0.0, 0.5), h: finite(h, 0.0).rem_euclid(360.0) }
    }

    fn lab(self) -> [f64; 3] {
        let r = self.h.to_radians();
        [self.l, self.c * r.cos(), self.c * r.sin()]
    }

    fn from_lab(lab: [f64; 3]) -> Oklch {
        let c = (lab[1] * lab[1] + lab[2] * lab[2]).sqrt();
        // Greys have no hue: report 0 rather than noise.
        if c < 1e-6 { Oklch::new(lab[0], 0.0, 0.0) } else { Oklch::new(lab[0], c, lab[2].atan2(lab[1]).to_degrees()) }
    }

    /// `c`'s OKLCH, reading its values in `profile` (alpha is ignored).
    pub fn from_color(c: Color, profile: ColorProfile) -> Oklch {
        let lin = [c.r, c.g, c.b].map(|v| decode(f64::from(v)));
        let lin = match profile {
            ColorProfile::Srgb => lin,
            ColorProfile::DisplayP3 => mul(&SRGB_FROM_P3, lin),
        };
        Oklch::from_lab(mul(&LAB_FROM_LMS, mul(&LMS_FROM_SRGB, lin).map(f64::cbrt)))
    }

    /// Gamma-encoded RGB in `profile`, unclipped.
    fn rgb(self, profile: ColorProfile) -> [f64; 3] {
        let lin = mul(&SRGB_FROM_LMS, mul(&LMS_FROM_LAB, self.lab()).map(|x| x * x * x));
        let lin = match profile {
            ColorProfile::Srgb => lin,
            ColorProfile::DisplayP3 => mul(&P3_FROM_SRGB, lin),
        };
        lin.map(encode)
    }

    /// Whether the colour fits `profile`'s gamut.
    pub fn in_gamut(self, profile: ColorProfile) -> bool {
        fits(self.rgb(profile))
    }

    /// The colour in `profile` with alpha `a`, gamut-mapped when it doesn't fit (see the module
    /// docs).
    pub fn to_color(self, profile: ColorProfile, a: f32) -> Color {
        let [r, g, b] = self.fit(profile);
        Color::new(r as f32, g as f32, b as f32, a)
    }

    /// Gamma-encoded RGB in `profile`, every channel 0..1 (CSS Color 4 gamut mapping).
    fn fit(self, profile: ColorProfile) -> [f64; 3] {
        let rgb = self.rgb(profile);
        if fits(rgb) {
            return clip(rgb);
        }
        if self.l >= 1.0 {
            return [1.0; 3];
        }
        if self.l <= 0.0 {
            return [0.0; 3];
        }
        const JND: f64 = 0.02;
        let mut clipped = clip(rgb);
        if delta_e(Oklch::from_color(color3(clipped), profile), self) < JND {
            return clipped;
        }
        let (mut lo, mut hi) = (0.0, self.c);
        for _ in 0..40 {
            if hi - lo < 1e-4 {
                break;
            }
            let cur = Oklch { c: (lo + hi) / 2.0, ..self };
            let raw = cur.rgb(profile);
            clipped = clip(raw);
            if fits(raw) {
                lo = cur.c;
                continue;
            }
            let e = delta_e(Oklch::from_color(color3(clipped), profile), cur);
            if e < JND {
                if JND - e < 1e-4 {
                    break;
                }
                lo = cur.c;
            } else {
                hi = cur.c;
            }
        }
        clipped
    }
}

const EPS: f64 = 1e-5;

fn fits(rgb: [f64; 3]) -> bool {
    rgb.iter().all(|v| *v >= -EPS && *v <= 1.0 + EPS)
}

fn clip(rgb: [f64; 3]) -> [f64; 3] {
    rgb.map(|v| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 })
}

fn color3(rgb: [f64; 3]) -> Color {
    Color::new(rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, 1.0)
}

/// Euclidean distance in OKLab.
fn delta_e(a: Oklch, b: Oklch) -> f64 {
    let (x, y) = (a.lab(), b.lab());
    ((x[0] - y[0]).powi(2) + (x[1] - y[1]).powi(2) + (x[2] - y[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn srgb_primaries_match_published_values() {
        let red = Oklch::from_color(Color::new(1.0, 0.0, 0.0, 1.0), ColorProfile::Srgb);
        assert!(close(red.l, 0.62796, 1e-3) && close(red.c, 0.25768, 1e-3) && close(red.h, 29.234, 0.05), "{red:?}");
        assert_eq!(red.to_color(ColorProfile::Srgb, 1.0).to_hex(), "FF0000");
        let white = Oklch::from_color(Color::WHITE, ColorProfile::Srgb);
        assert!(close(white.l, 1.0, 1e-4) && white.c < 1e-4);
        let blue = Oklch::from_color(Color::from_hex("#0000FF").unwrap_or_default(), ColorProfile::Srgb);
        assert!(close(blue.l, 0.45201, 1e-3) && close(blue.h, 264.052, 0.1), "{blue:?}");
    }

    #[test]
    fn display_p3_is_wider() {
        let red = Oklch::from_color(Color::new(1.0, 0.0, 0.0, 1.0), ColorProfile::DisplayP3);
        assert!(close(red.l, 0.64857, 2e-3) && close(red.c, 0.29948, 2e-3), "{red:?}");
        assert!(red.in_gamut(ColorProfile::DisplayP3) && !red.in_gamut(ColorProfile::Srgb));
        assert_eq!(red.to_color(ColorProfile::DisplayP3, 1.0).to_hex(), "FF0000");
    }

    #[test]
    fn round_trips_and_keeps_alpha() {
        for hex in ["#3366FF", "#00A699", "#7B0051", "#F5F5F5", "#111111"] {
            let c = Color::from_hex(hex).unwrap_or_default();
            for p in ColorProfile::ALL {
                assert_eq!(Oklch::from_color(c, p).to_color(p, 0.5).to_css_hex(), format!("{hex}80"), "{hex} {p:?}");
            }
        }
    }

    #[test]
    fn out_of_gamut_maps_inside_keeping_lightness_and_hue() {
        let want = Oklch::new(0.7, 0.35, 150.0);
        assert!(!want.in_gamut(ColorProfile::Srgb));
        let got = Oklch::from_color(want.to_color(ColorProfile::Srgb, 1.0), ColorProfile::Srgb);
        assert!(got.c < 0.35 && close(got.l, 0.7, 0.03) && close(got.h, 150.0, 4.0), "{got:?}");
        let p3 = Oklch::from_color(want.to_color(ColorProfile::DisplayP3, 1.0), ColorProfile::DisplayP3);
        assert!(p3.c > got.c, "{p3:?}");
        // The extremes and junk.
        assert_eq!(Oklch::new(1.0, 0.3, 40.0).to_color(ColorProfile::Srgb, 1.0).to_hex(), "FFFFFF");
        assert_eq!(Oklch::new(0.0, 0.3, 40.0).to_color(ColorProfile::Srgb, 1.0).to_hex(), "000000");
        assert_eq!(Oklch::new(f64::NAN, f64::INFINITY, f64::NAN), Oklch { l: 0.0, c: 0.0, h: 0.0 });
    }
}
