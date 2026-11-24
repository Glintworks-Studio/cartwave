//! Minimal `f64` math for `no_std`. Used only at construction time (kernel and
//! filter design, pitch tables, sample synthesis), never per sample. Tested
//! against `std`.

pub const PI: f64 = core::f64::consts::PI;
pub const LN2: f64 = core::f64::consts::LN_2;

/// Floor for |x| < 2^62.
pub fn floor(x: f64) -> f64 {
    let t = x as i64 as f64;
    if t > x {
        t - 1.0
    } else {
        t
    }
}

pub fn round(x: f64) -> f64 {
    floor(x + 0.5)
}

pub fn abs(x: f64) -> f64 {
    if x < 0.0 {
        -x
    } else {
        x
    }
}

fn pow2i(k: i32) -> f64 {
    debug_assert!((-1022..=1023).contains(&k));
    f64::from_bits(((k + 1023) as u64) << 52)
}

pub fn exp(x: f64) -> f64 {
    if x < -700.0 {
        return 0.0;
    }
    if x > 700.0 {
        return f64::INFINITY;
    }
    let k = round(x / LN2);
    let r = x - k * LN2;
    let mut term = 1.0;
    let mut sum = 1.0;
    for i in 1..=24 {
        term *= r / i as f64;
        sum += term;
    }
    sum * pow2i(k as i32)
}

/// Natural log for finite, normal x > 0.
pub fn ln(x: f64) -> f64 {
    debug_assert!(x > 0.0);
    let bits = x.to_bits();
    let e = ((bits >> 52) & 0x7ff) as i32 - 1023;
    let m = f64::from_bits((bits & ((1u64 << 52) - 1)) | (1023u64 << 52));
    // ln(m) = 2 atanh(s), s = (m-1)/(m+1) <= 1/3
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let mut term = s;
    let mut sum = 0.0;
    for i in 0..40 {
        sum += term / (2 * i + 1) as f64;
        term *= s2;
    }
    2.0 * sum + e as f64 * LN2
}

pub fn log2(x: f64) -> f64 {
    ln(x) / LN2
}

pub fn exp2(x: f64) -> f64 {
    exp(x * LN2)
}

fn reduce(x: f64) -> f64 {
    let tau = 2.0 * PI;
    let mut r = x - floor(x / tau) * tau; // [0, tau)
    if r > PI {
        r -= tau;
    }
    r
}

pub fn sin(x: f64) -> f64 {
    let r = reduce(x);
    let r2 = r * r;
    let mut term = r;
    let mut sum = r;
    for i in 1..30 {
        term *= -r2 / ((2 * i) as f64 * (2 * i + 1) as f64);
        sum += term;
    }
    sum
}

pub fn cos(x: f64) -> f64 {
    let r = reduce(x);
    let r2 = r * r;
    let mut term = 1.0;
    let mut sum = 1.0;
    for i in 1..30 {
        term *= -r2 / ((2 * i - 1) as f64 * (2 * i) as f64);
        sum += term;
    }
    sum
}

pub fn tan(x: f64) -> f64 {
    sin(x) / cos(x)
}

pub fn sqrt(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut y = f64::from_bits((x.to_bits() >> 1) + (1023u64 << 51));
    for _ in 0..8 {
        y = 0.5 * (y + x / y);
    }
    y
}

/// Zeroth-order modified Bessel function of the first kind (Kaiser window).
pub fn bessel_i0(x: f64) -> f64 {
    let h = x / 2.0;
    let mut term = 1.0;
    let mut sum = 1.0;
    for k in 1..80 {
        term *= h / k as f64;
        let t2 = term * term;
        sum += t2;
        if t2 < sum * 1e-17 {
            break;
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        abs(a - b) <= tol * (1.0 + abs(b))
    }

    #[test]
    fn matches_std() {
        for i in -200..200 {
            let x = i as f64 * 0.173;
            assert!(close(exp(x / 10.0), (x / 10.0).exp(), 1e-13), "exp {x}");
            assert!(close(sin(x), x.sin(), 1e-12), "sin {x}");
            assert!(close(cos(x), x.cos(), 1e-12), "cos {x}");
            assert!(close(floor(x), x.floor(), 0.0), "floor {x}");
            let p = abs(x) + 1e-3;
            assert!(close(ln(p), p.ln(), 1e-13), "ln {p}");
            assert!(close(sqrt(p), p.sqrt(), 1e-14), "sqrt {p}");
        }
        assert!(close(tan(0.3), 0.3f64.tan(), 1e-13));
        assert!(close(exp2(-7.25), 2f64.powf(-7.25), 1e-13));
        // I0(1) = 1.2660658777520082
        assert!(close(bessel_i0(1.0), 1.266_065_877_752_008_2, 1e-14));
    }
}
