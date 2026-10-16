//! Trig for the simulation, from the pure-Rust `libm` crate so results are
//! bit-identical on every platform. The standard `f32::sin` and friends call
//! the system maths library, whose last bits differ between Linux, Windows
//! and macOS, and a replay would slowly drift apart. Sim code uses these
//! (a test in `tests/determinism.rs` checks); drawing code may use either.

pub fn sin(x: f32) -> f32 {
    libm::sinf(x)
}

pub fn cos(x: f32) -> f32 {
    libm::cosf(x)
}

pub fn atan2(y: f32, x: f32) -> f32 {
    libm::atan2f(y, x)
}

#[cfg(test)]
mod tests {
    #[test]
    fn matches_the_usual_functions_closely() {
        for i in -100..100 {
            let x = i as f32 * 0.137;
            assert!((super::sin(x) - x.sin()).abs() < 1e-6);
            assert!((super::cos(x) - x.cos()).abs() < 1e-6);
            assert!((super::atan2(x, 1.3) - x.atan2(1.3)).abs() < 1e-6);
        }
    }
}
