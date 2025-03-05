#[derive(Debug, Clone)]
pub struct NoitaRng(i64);

impl NoitaRng {
    /// The random function itself is super standard, the secret sauce was
    /// getting the state from the world seed and position
    pub fn random(&mut self) -> f64 {
        let hi = self.0 / 127773;
        let lo = self.0 - hi * 127773;
        self.0 = lo * 16807 - hi * 2836;
        if self.0 <= 0 {
            self.0 += 0x7fffffff;
        }
        self.0 as f64 * 4.656612875e-10
    }

    pub fn from_pos(seed_plus_ng: u32, x: f64, y: f64) -> Self {
        let xo = x + ((seed_plus_ng ^ 0x93262e6f) & 0xfff) as f64;
        let yo = y + (((seed_plus_ng ^ 0x93262e6f) >> 12) & 0xfff) as f64;

        let xi = to_int_kinda(xo * 134217727.0);
        let yi = to_int_kinda(if yo.abs() >= 102400.0 || xo.abs() <= 1.0 {
            yo * 134217727.0
        } else {
            yo * (yo * 3483.328 + xi as f64)
        });

        let mixed = mix(xi as i32, yi as i32, seed_plus_ng);

        let mut state = (mixed as f64) / 4294967295.0 * 2147483639.0 + 1.0;
        if state >= 2147483647.0 {
            state *= 0.5;
        }

        let mut rng = Self(state as i64);
        rng.random();

        for _ in 0..(seed_plus_ng & 3) {
            rng.random();
        }
        rng
    }
}

// wrapping_sub soup
fn mix(a: i32, b: i32, c: u32) -> u32 {
    let mut x = (a.wrapping_sub(b) as u32).wrapping_sub(c) ^ (c >> 13);
    let mut y = (b as u32).wrapping_sub(x).wrapping_sub(c) ^ (x << 8);
    let mut z = c.wrapping_sub(x).wrapping_sub(y) ^ (y >> 13);
    x = x.wrapping_sub(y).wrapping_sub(z) ^ (z >> 12);
    y = y.wrapping_sub(x).wrapping_sub(z) ^ (x << 16);
    z = z.wrapping_sub(x).wrapping_sub(y) ^ (y >> 5);
    x = x.wrapping_sub(y).wrapping_sub(z) ^ (z >> 3);
    y = y.wrapping_sub(x).wrapping_sub(z) ^ (x << 10);
    z.wrapping_sub(x).wrapping_sub(y) ^ (y >> 15)
}

// pretty sure this was some bog standard double->int conversion function of stl or something
fn to_int_kinda(input: f64) -> u64 {
    // let is_normal_finite = ((bits >> 32) & 0x7fff_ffff) < 0x7ff0_0000;
    let is_normal_finite = input.is_finite();
    let valid_range = (-9.223372036854776e18..9.223372036854776e18).contains(&input);

    // could remove this check ig, I've never seen that warning
    if !is_normal_finite || !valid_range {
        tracing::warn!("invalid float received");
        return (-0.0f64).to_bits();
    }

    let abs_bits = input.to_bits() & !(1 << 63);
    let in_abs = f64::from_bits(abs_bits);
    if in_abs == 0.0 {
        return 0;
    }

    let exponent = abs_bits >> 52;
    let norm_mantissa = (abs_bits & ((1 << 52) - 1)) | (1 << 52);

    let shift = 0x433 - exponent as i32;
    let mut result = if shift > 0 {
        norm_mantissa >> (shift & 63)
    } else {
        norm_mantissa << (-shift & 63)
    };

    // restore the sign lol
    if input != in_abs {
        result = result.wrapping_neg();
    }

    result & 0xffff_ffff
}
