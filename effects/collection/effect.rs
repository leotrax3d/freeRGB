//! API-v1 guests. Each cfg selects a separate algorithm; unused branches are removed.
//! No allocator, imports, OS state or frame-count-dependent randomness.
#![no_std]
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
static mut FRAME: [u8; 128 * 64 * 3] = [0; 128 * 64 * 3];
static mut FIELD: [[f32; 3]; 32 * 16] = [[0.; 3]; 32 * 16];
static mut PARAMS: [f32; 6] = [1., 0.8, 0., 0., 0., 0.];
const TAU: f32 = core::f32::consts::TAU;
fn fract(x: f32) -> f32 {
    x - x.floor()
}
// no_std floor is provided explicitly to avoid platform libm / WASI imports.
trait Floor {
    fn floor(self) -> f32;
}
impl Floor for f32 {
    fn floor(self) -> f32 {
        let i = self as i32 as f32;
        if i > self {
            i - 1.
        } else {
            i
        }
    }
}
fn sat(x: f32) -> f32 {
    x.clamp(0., 1.)
}
fn wave(x: f32) -> f32 {
    let a = (fract(x + 0.5) - 0.5) * TAU;
    // Fast sine approximation, with exact zero crossings.
    let b = 1.273_239_5 * a - 0.405_284_73 * a * a.abs();
    0.225 * (b * b.abs() - b) + b
}
fn cos(x: f32) -> f32 {
    wave(x + 0.25)
}
fn hash(x: u32) -> f32 {
    let mut n = x.wrapping_add(0x9e3779b9);
    n = (n ^ (n >> 16)).wrapping_mul(0x21f0aaad);
    n = (n ^ (n >> 15)).wrapping_mul(0x735a2d97);
    ((n ^ (n >> 15)) >> 8) as f32 / 16777216.
}
fn noise(x: f32, s: u32) -> f32 {
    let i = x.floor() as i32;
    let f = fract(x);
    let f = f * f * (3. - 2. * f);
    let a = hash((i as u32).wrapping_add(s));
    a + (hash((i as u32).wrapping_add(1).wrapping_add(s)) - a) * f
}
fn sqrt(x: f32) -> f32 {
    if x <= 0. {
        return 0.;
    }
    let a = f32::from_bits((x.to_bits() >> 1) + 0x1fc00000);
    0.5 * (a + x / a)
}
fn angle(x: f32, y: f32) -> f32 {
    let a = y.abs() + 0.00001;
    let (r, b) = if x >= 0. {
        ((x - a) / (x + a), 0.125)
    } else {
        ((x + a) / (a - x), 0.375)
    };
    let v = b + ((0.1963 * r * r - 0.9817) * r) / TAU;
    if y < 0. {
        -v
    } else {
        v
    }
}
fn color(h: f32, v: f32) -> [f32; 3] {
    let h = fract(h);
    [
        sat((fract(h + 1.) * 6. - 3.).abs() - 1.) * v,
        sat(2. - (fract(h) * 6. - 2.).abs()) * v,
        sat(2. - (fract(h) * 6. - 4.).abs()) * v,
    ]
}
fn mix(a: [f32; 3], b: [f32; 3], f: f32) -> [f32; 3] {
    let f = sat(f);
    [
        a[0] + (b[0] - a[0]) * f,
        a[1] + (b[1] - a[1]) * f,
        a[2] + (b[2] - a[2]) * f,
    ]
}
fn spot(d: f32, w: f32) -> f32 {
    let a = sat(1. - d / w);
    a * a
}
#[no_mangle]
pub extern "C" fn set_param(index: i32, value: f64) {
    if !(0..6).contains(&index) || !value.is_finite() {
        return;
    }
    let i = index as usize;
    let (lo, hi) = match i {
        0 => (0.1, 3.),
        1 => (0., 1.),
        _ => (0., 65535.),
    };
    unsafe {
        PARAMS[i] = (value as f32).clamp(lo, hi);
    }
}
fn field(x: f32, y: f32, t: f32, p: [f32; 6], one: bool) -> [f32; 3] {
    let a = p[2];
    let b = p[3];
    let c = p[4];
    let seed = c as u32;
    let u = x - 0.5;
    let v = y - 0.5;
    let r = sqrt(u * u + v * v);
    if cfg!(effect = "polarlicht") {
        let mut out = [0.; 3];
        for i in 0..(a as u32).clamp(2, 6) {
            let k = i as f32;
            let axis = if one { x } else { y };
            let centre = 0.5
                + 0.3 * wave(x * 0.7 + t * 0.07 + k * 0.19)
                + 0.13 * wave(x * 2. - t * 0.09 + k);
            let light = spot((axis - centre).abs(), b.max(0.08));
            let rgb = color(c / 360. + k * 0.08 + x * 0.1, light * 0.55);
            for j in 0..3 {
                out[j] += rgb[j];
            }
        }
        return out;
    }
    if cfg!(effect = "feuer") {
        let height = if one { x } else { 1. - y };
        let turbulence = noise(x * 12. + wave(t * 0.3) * b * 2., seed)
            + noise(x * 7. + height * 4. - t * 3., seed + 7);
        let heat = sat((a * 1.4 - height + 0.22 * (turbulence - 1.) * b) * 1.6);
        return [heat, sat(heat * heat * 1.2 - 0.12), sat(heat * 3. - 2.5)];
    }
    if cfg!(effect = "gewitter") {
        let clock = t * a.max(0.2);
        let event = clock.floor() as u32;
        let age = fract(clock);
        let strike = hash(event.wrapping_add(seed)) > 0.27;
        let bolt = hash(event.wrapping_add(seed + 15)) + 0.08 * wave(y * 5. + event as f32);
        let glow = if strike {
            sat(1. - age / b.max(0.05))
        } else {
            0.
        };
        let flash = if age < 0.05 || (age > 0.12 && age < 0.16) {
            1.
        } else {
            0.
        };
        let l = glow * (0.13 + spot((x - bolt).abs(), 0.055) * flash);
        let sky = 0.008 * wave(x * 0.7 + t * 0.04);
        return [0.015 + l + sky, 0.025 + l + sky, 0.06 + l + sky];
    }
    if cfg!(effect = "regentropfen") {
        let mut light = 0.;
        for i in 0..(a as u32).clamp(1, 6) {
            let q = t * 0.35 + i as f32 * 0.73;
            let event = q.floor() as u32;
            let h = seed.wrapping_add(i * 71).wrapping_add(event * 19);
            let dx = x - hash(h);
            let dy = if one { 0. } else { y - hash(h + 1) };
            let age = fract(q);
            let dist = sqrt(dx * dx + dy * dy);
            light += spot((dist - age * 0.9).abs(), b.max(0.02)) * (1. - age);
        }
        return [0.02 * light, 0.03 + 0.5 * light, 0.08 + light];
    }
    if cfg!(effect = "sternenhimmel") {
        let gx = (x * 24.).floor() as u32;
        let gy = (y * 12.).floor() as u32;
        let s = seed.wrapping_add(gx * 31 + gy * 719);
        let h = hash(s);
        let age = fract(t / b.max(0.5) * (0.6 + hash(s + 2)) + hash(s + 3));
        let life = sat(wave(age * 0.5)) * sat(wave(age * 0.5));
        let d = (fract(x * 24.) - hash(s + 4)).abs()
            + if one {
                0.
            } else {
                (fract(y * 12.) - hash(s + 5)).abs()
            };
        let l = if h < a {
            spot(d, 0.8) * life * (0.3 + 0.7 * hash(s + 6))
        } else {
            0.
        };
        return [0.012 + l * 0.8, 0.015 + l * 0.9, 0.04 + l];
    }
    if cfg!(effect = "meteorschauer") {
        let dx = cos(a / 360.);
        let dy = wave(a / 360.);
        let mut l = 0.;
        for i in 0..4 {
            let s = p[5] as u32 + i * 137;
            let age = fract(t * 0.35 + hash(s));
            let xx = x - (0.5 + dx * (age * 2. - 1.) + (hash(s + 2) - 0.5) * b);
            let yy = if one {
                0.
            } else {
                y - (0.5 + dy * (age * 2. - 1.) + (hash(s + 3) - 0.5) * b)
            };
            let along = xx * dx + yy * dy;
            let cross = if one { 0. } else { (xx * dy - yy * dx).abs() };
            if along <= 0.04 && along >= -c {
                l += spot(cross, 0.07) * sat(1. + along / c.max(0.1));
            }
        }
        return [l, l * 0.55, 0.035 + l * 0.4];
    }
    if cfg!(effect = "ozean") {
        let n = wave(x * c + t * 0.13 + wave(y * 2. - t * 0.1) * 0.2) * 0.5
            + wave(y * c - x - t * 0.21) * 0.3;
        let depth = sat(0.5 + n * 0.5);
        let foam = sat((n - 0.65 + b * 0.35) * 6.) * b;
        return [
            0.01 + foam,
            0.08 + depth * 0.45 * (1. - a * 0.5) + foam,
            0.15 + depth * 0.6 + foam,
        ];
    }
    if cfg!(effect = "lava") {
        let mut f = 0.;
        for i in 0..4 {
            let s = seed + i * 17;
            let cx = 0.5 + 0.4 * wave(t * 0.035 + hash(s));
            let cy = 0.5 + 0.4 * wave(t * 0.043 + hash(s + 1));
            let d = (x - cx) * (x - cx) + if one { 0. } else { (y - cy) * (y - cy) };
            f += a * 0.025 / (d + 0.025);
        }
        let l = sat((f - 0.5) * b);
        return [l, sat(l * l - 0.1) * 0.5, 0.015 + l * 0.02];
    }
    if cfg!(effect = "plasma") {
        let n = (wave(x * a + t * 0.17)
            + wave(y * a - t * 0.13)
            + wave((x + y) * a * 0.7 + t * 0.11)
            + wave(r * a - t * 0.2))
            * 0.125
            + 0.5;
        return color(b / 360. + n, n * 0.7 + 0.3);
    }
    if cfg!(effect = "galaxie") {
        let arm = wave(angle(u, v) * a - r * b + t * 0.12);
        let gas = sat(1. - r * 1.5) * (0.15 + 0.65 * sat(arm));
        let s = seed + ((x * 50.) as u32) * 31 + ((y * 30.) as u32) * 719;
        let star = if hash(s) > 0.975 {
            sat(wave(t * 0.3 + hash(s + 1)))
        } else {
            0.
        };
        return [gas * 0.6 + star, gas * 0.25 + star, gas + star];
    }
    if cfg!(effect = "schwarzes-loch") {
        let radius = if one { (x - 0.5).abs() } else { r };
        if radius < a {
            return [0.; 3];
        }
        let ring = spot((radius - a - 0.06).abs(), 0.065);
        let spiral = sat(wave(angle(u, v) * 3. + radius * b + t * 0.2)) * 0.35;
        return color(
            c / 360. + radius * 0.35,
            (ring + spiral) * sat(2. - radius * 2.),
        );
    }
    if cfg!(effect = "konfetti") {
        let xx = x + t * 0.08;
        let yy = y - t * 0.18;
        let gx = (xx * 16.).floor() as u32;
        let gy = (yy * 10. + 10000.).floor() as u32;
        let s = seed.wrapping_add(gx * 31).wrapping_add(gy * 719);
        let age = fract(t / b.max(0.3) + hash(s + 1));
        let shape = spot((fract(xx * 16.) - 0.5).abs(), 0.35)
            * if one {
                1.
            } else {
                spot((fract(yy * 10.) - 0.5).abs(), 0.35)
            };
        return color(
            hash(s + 2),
            if hash(s) < a { shape * (1. - age) } else { 0. },
        );
    }
    if cfg!(effect = "gluehwuermchen") {
        let gx = (x * 6.).floor() as u32;
        let gy = (y * 4.).floor() as u32;
        let s = seed + gx * 31 + gy * 719;
        let dx = fract(x * 6.) - (0.5 + 0.25 * wave(t * 0.07 + hash(s + 1)));
        let dy = if one {
            0.
        } else {
            fract(y * 4.) - (0.5 + 0.25 * wave(t * 0.09 + hash(s + 2)))
        };
        let l = if hash(s) < a {
            spot(sqrt(dx * dx + dy * dy), b) * (0.15 + 0.85 * sat(wave(t * 0.18 + hash(s + 3))))
        } else {
            0.
        };
        return [l * 0.7, l, l * 0.15];
    }
    if cfg!(effect = "neonroehren") {
        let segment = (x * a).floor();
        let q = fract(t * 0.12 + segment * 0.137);
        let startup = sat(q * 12.);
        let flicker = if q < 0.12 {
            1. - b * hash((t * 35.) as u32 + segment as u32 * 19)
        } else {
            1.
        };
        let tube = if fract(x * a) < 0.12 {
            0.
        } else {
            if one {
                1.
            } else {
                spot((y - 0.5).abs(), 0.36)
            }
        };
        return color(c / 360. + segment * 0.17, tube * startup * flicker);
    }
    if cfg!(effect = "cyberpunk") {
        let lane = (y * b).floor();
        let tick = (t * 8.).floor() as u32;
        let s = seed + tick * 17;
        let glitch = hash(s) < a * 0.25;
        let head = fract(x - t * (0.2 + lane * 0.03) + lane * 0.21);
        let hue = if (x * 5. + lane + t * 0.3).floor() as i32 % 2 == 0 {
            0.88
        } else {
            0.5
        };
        return if glitch {
            color(hash(s + 1), if fract(x * 17.) > 0.5 { 1. } else { 0. })
        } else {
            color(hue, if head < 0.14 { 1. } else { 0.13 })
        };
    }
    if cfg!(effect = "matrixregen") {
        let col = (x * 18.).floor() as u32;
        let s = seed + col * 719;
        let fall = if one { x } else { y };
        let d = fract(t * (0.15 + hash(s + 1) * 0.3) - fall + hash(s + 2));
        let l = if hash(s) < b && d < a {
            sat(1. - d / a.max(0.1))
        } else {
            0.
        };
        let glyph = 0.45 + 0.55 * hash(s + ((fall * 24.) as u32) * 13 + (t * 5.) as u32);
        return [sat(l * 5. - 4.), l * glyph, l * 0.32];
    }
    if cfg!(effect = "radar") {
        let theta = if one { x } else { fract(angle(u, v)) };
        let sweep = fract(t * 0.13);
        let behind = fract(sweep - theta);
        let beam = spot(behind, a);
        let s = seed + ((x * 16.) as u32) * 31 + ((y * 10.) as u32) * 719;
        let target = if hash(s) > 0.94 {
            sat(1. - behind / b.max(0.1))
        } else {
            0.
        };
        let grid = if ((r * 8.).floor() - r * 8.).abs() < 0.07 {
            0.08
        } else {
            0.
        };
        return [
            target * 0.4,
            beam * 0.7 + target + grid,
            0.06 + beam * 0.2 + target * 0.3,
        ];
    }
    if cfg!(effect = "herzschlag") {
        let phase = fract(t * 0.6);
        let active = (1. - a).max(0.3);
        let z = phase / active;
        let pulse = spot((z - 0.2).abs(), 0.18 / b.max(1.).sqrt_approx())
            + spot((z - 0.55).abs(), 0.23 / b.max(1.).sqrt_approx()) * 0.7;
        return color(c / 360., 0.025 + pulse * 0.975);
    }
    if cfg!(effect = "atemlicht") {
        let curve = (0.5 - 0.5 * cos(t * 0.16)).max(0.001);
        let light = curve / (curve + (1. - curve) * a);
        return color(c / 360. + b * t * 0.035, light);
    }
    if cfg!(effect = "pendel") {
        let axis = if one {
            x
        } else {
            0.5 + u * cos(b / 360.) + v * wave(b / 360.)
        };
        return color(
            c / 360.,
            0.015 + spot((axis - (0.5 + 0.45 * wave(t * 0.24))).abs(), a),
        );
    }
    if cfg!(effect = "kaleidoskop") {
        let theta = angle(u, v);
        let mirror = wave(theta * a + t * 0.08).abs();
        let n = wave(r * b * 2. + mirror - t * 0.15) * 0.5 + 0.5;
        return color(c / 360. + mirror * 0.4 + r * 0.3, n);
    }
    if cfg!(effect = "schachbrett") {
        let dx = cos(c / 360.);
        let dy = wave(c / 360.);
        let xx = (x * a + t * 0.3 * dx).floor();
        let yy = if one {
            0.
        } else {
            (y * a + t * 0.3 * dy).floor()
        };
        let parity = fract((xx + yy) * 0.5) * 2.;
        let q = fract(t * 0.2);
        let fade = sat((q - 0.5) / b.max(0.02) + 0.5);
        return mix(
            [0.5, 0.08, 0.85],
            [1., 0.6, 0.12],
            if parity > 0.5 { fade } else { 1. - fade },
        );
    }
    if cfg!(effect = "farbfluss") {
        let axis = if one {
            x * cos(a / 360.)
        } else {
            x * cos(a / 360.) + y * wave(a / 360.)
        };
        let q = fract(axis * b - t * 0.15);
        let blend = fract(q * 3.);
        let k = (q * 3.).floor();
        return mix(
            color(c / 360. + k / 3., 1.),
            color(c / 360. + (k + 1.) / 3., 1.),
            blend * blend * (3. - 2. * blend),
        );
    }
    [0.; 3]
}
trait SqrtApprox {
    fn sqrt_approx(self) -> f32;
}
impl SqrtApprox for f32 {
    fn sqrt_approx(self) -> f32 {
        sqrt(self)
    }
}
// Gray-Scott: two concentration buffers, fixed 8 Hz steps, toroidal boundary.
// Replay a seeded 32-step growth trajectory forward/backward over eight seconds.
// This bounds arbitrary seeking and visibly dissolves patterns without a reset jump.
static mut CHEM: [[f32; 2]; 128] = [[1., 0.]; 128];
static mut NEXT: [[f32; 2]; 128] = [[1., 0.]; 128];
static mut RD_TICK: i32 = -1;
static mut RD_EPOCH: i32 = -1;
static mut RD_CONFIG: [f32; 3] = [0.; 3];
unsafe fn reaction(t: f32, p: [f32; 6]) {
    let epoch = 0;
    let phase = fract(t / 8.);
    let tick = (if phase < 0.5 {
        phase * 64.
    } else {
        (1. - phase) * 64.
    })
    .min(31.) as i32;
    let config = [p[2], p[3], p[4]];
    if epoch != RD_EPOCH || tick < RD_TICK || config != RD_CONFIG {
        for i in 0..128 {
            let seed = hash(i as u32 + p[4] as u32 + (epoch as u32).wrapping_mul(97));
            CHEM[i] = if seed > 0.84 { [0.4, 0.8] } else { [1., 0.] };
        }
        RD_EPOCH = epoch;
        RD_TICK = 0;
        RD_CONFIG = config;
    }
    while RD_TICK < tick {
        for y in 0..8 {
            for x in 0..16 {
                let i = y * 16 + x;
                let [a, b] = CHEM[i];
                let l = CHEM[y * 16 + (x + 15) % 16];
                let r = CHEM[y * 16 + (x + 1) % 16];
                let u = CHEM[((y + 7) % 8) * 16 + x];
                let d = CHEM[((y + 1) % 8) * 16 + x];
                let ab = a * b * b;
                NEXT[i] = [
                    sat(a + 0.18 * (l[0] + r[0] + u[0] + d[0] - 4. * a) - ab + p[2] * (1. - a)),
                    sat(b + 0.09 * (l[1] + r[1] + u[1] + d[1] - 4. * b) + ab - (p[2] + p[3]) * b),
                ];
            }
        }
        CHEM = NEXT;
        RD_TICK += 1;
    }
}
unsafe fn splat(cx: f32, cy: f32, hue: f32, light: f32, gw: usize, gh: usize, one: bool) {
    let xx = cx * (gw - 1) as f32;
    let yy = if one { 0. } else { cy * (gh - 1) as f32 };
    let rgb = color(hue, light);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let x = xx as i32 + dx;
            let y = yy as i32 + dy;
            if x >= 0 && y >= 0 && x < gw as i32 && y < gh as i32 {
                let weight = spot(
                    (x as f32 - xx).abs() + if one { 0. } else { (y as f32 - yy).abs() },
                    1.8,
                );
                let i = y as usize * gw + x as usize;
                for (j, c) in rgb.iter().enumerate() {
                    FIELD[i][j] += c * weight;
                }
            }
        }
    }
}
#[no_mangle]
pub extern "C" fn render(time: f64, width: i32, height: i32) -> i32 {
    if !time.is_finite() || time < 0. || width <= 0 || height <= 0 || width > 128 || height > 64 {
        return -1;
    }
    let w = width as usize;
    let h = height as usize;
    let single = w * h == 1;
    let one = (w == 1 || h == 1) && !single;
    let gw = if single {
        16
    } else if one {
        32
    } else {
        w.min(32)
    };
    let gh = if single {
        8
    } else if one {
        1
    } else {
        h.min(16)
    };
    unsafe {
        let p = PARAMS;
        let t = ((time * p[0] as f64) % 65536.) as f32;
        if cfg!(effect = "reaktionsdiffusion") {
            reaction(t, p);
        }
        for gy in 0..gh {
            for gx in 0..gw {
                let x = if gw == 1 {
                    0.5
                } else {
                    gx as f32 / (gw - 1) as f32
                };
                let y = if gh == 1 {
                    0.5
                } else {
                    gy as f32 / (gh - 1) as f32
                };
                FIELD[gy * gw + gx] = if cfg!(effect = "reaktionsdiffusion") {
                    let i = (y * 7.) as usize * 16 + (x * 15.) as usize;
                    let b = CHEM[i][1];
                    mix([0.025, 0.06, 0.16], [0.7, 1., 0.65], sat(b * 2.))
                } else {
                    field(x, y, t, p, one)
                };
            }
        }
        if cfg!(effect = "feuerwerk") {
            let seed = p[4] as u32;
            let clock = t * 0.3;
            let event = clock.floor() as u32;
            let age = fract(clock);
            let origin = 0.2 + 0.6 * hash(seed.wrapping_add(event * 19));
            let hue = hash(seed.wrapping_add(event * 19 + 1));
            if age < 0.3 {
                let q = age / 0.3;
                splat(origin, 1. - q * 0.65, hue, 1., gw, gh, one);
                if one {
                    splat(q, 0., hue, 1., gw, gh, one);
                }
            } else {
                let q = (age - 0.3) / 0.7;
                for i in 0..(p[2] as u32).clamp(6, 24) {
                    let theta = i as f32 / p[2] + hash(seed + i) * 0.03;
                    let radius = q * p[3] * 0.6;
                    let x = origin + cos(theta) * radius;
                    let y = 0.35 + wave(theta) * radius + q * q * 0.4;
                    splat(x, y, hue + i as f32 * 0.013, 1. - q, gw, gh, one);
                }
            }
        }
        // Average the full field for a single LED and area sample short strips.
        // Tiny layouts retain energy instead of missing every sparse particle.
        if single {
            for j in 0..3 {
                let mut sum = 0.;
                for i in 0..gw * gh {
                    sum += FIELD[i][j];
                }
                FRAME[j] = (sat(sum / (gw * gh) as f32) * p[1] * 255.) as u8;
            }
            return core::ptr::addr_of_mut!(FRAME) as i32;
        }
        if one && w * h < 8 {
            let n = w * h;
            for led in 0..n {
                let start = led * gw / n;
                let end = (led + 1) * gw / n;
                for j in 0..3 {
                    let mut sum = 0.;
                    for i in start..end {
                        sum += FIELD[i][j];
                    }
                    FRAME[led * 3 + j] = (sat(sum / (end - start) as f32) * p[1] * 255.) as u8;
                }
            }
            return core::ptr::addr_of_mut!(FRAME) as i32;
        }
        // Bilinear sampling avoids block edges while keeping procedural work bounded.
        for y in 0..h {
            for x in 0..w {
                let fx = if one {
                    if w > 1 {
                        x as f32 / (w - 1) as f32
                    } else if h > 1 {
                        y as f32 / (h - 1) as f32
                    } else {
                        0.5
                    }
                } else {
                    if w > 1 {
                        x as f32 / (w - 1) as f32
                    } else {
                        0.5
                    }
                } * (gw - 1) as f32;
                let fy = if one {
                    0.
                } else {
                    y as f32 / (h - 1).max(1) as f32 * (gh - 1) as f32
                };
                let ix = fx as usize;
                let iy = fy as usize;
                let jx = (ix + 1).min(gw - 1);
                let jy = (iy + 1).min(gh - 1);
                let ax = fx - ix as f32;
                let ay = fy - iy as f32;
                let top = FIELD[iy * gw + ix];
                let right = FIELD[iy * gw + jx];
                let bottom = FIELD[jy * gw + ix];
                let corner = FIELD[jy * gw + jx];
                for j in 0..3 {
                    let c = top[j] + (right[j] - top[j]) * ax;
                    let d = bottom[j] + (corner[j] - bottom[j]) * ax;
                    FRAME[(y * w + x) * 3 + j] = (sat(c + (d - c) * ay) * p[1] * 255.) as u8;
                }
            }
        }
        core::ptr::addr_of_mut!(FRAME) as i32
    }
}
