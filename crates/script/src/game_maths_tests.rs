//! Lust's `Maths` in a running game works out its sines, exponentials,
//! arctangents, logs and powers with `libm` (`exact_maths`), so a game's
//! numbers are the same to the last bit on every PC: what the two-PC
//! check's `Director` object found differing on Windows.

use super::tests::{TestWorld, compiled, unhurried};
use reflect::Value;

const WAVES: &str = "object Waves {\n\
        has {\n\
            float t = 0;\n\
            float wave = 0;\n\
            float power = 0;\n\
            float logged = 0;\n\
            float turned = 0;\n\
        }\n\
        method update(float dt) {\n\
            t = t + dt;\n\
            wave = Maths:sin(t * 3.1) * Maths:exp(0 - t * 0.2) + Maths:atan2(t, 2) * 0.1;\n\
            power = Maths:power(1.7, t + 0.3);\n\
            logged = Maths:log(t + 1.5);\n\
            turned = Maths:cos(t) + Maths:tan(t * 0.5) + Maths:asin(0.3) + Maths:acos(0.2) + Maths:atan(t);\n\
        }\n\
    }\n\
    pass Waves;\nend\n";

#[test]
fn lust_s_maths_gives_libm_s_bits() {
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(1);
    game.load(&compiled(WAVES), &mut world).unwrap();
    let dt = 1.0 / 60.0;
    let mut t = 0.0_f64;
    for _ in 0..9 {
        assert!(game.tick(dt, &mut world).failures.is_empty());
        t += dt;
    }
    let values = game.object_values();
    let (_, waves) = values.iter().find(|(name, _)| name == "Waves").unwrap();
    let float = |field: &str| match waves.get(field) {
        Some(Value::Float(v)) => *v,
        other => panic!("{field}: {other:?}"),
    };
    assert_eq!(float("t").to_bits(), t.to_bits());
    let wave = libm::sin(t * 3.1) * libm::exp(0.0 - t * 0.2) + libm::atan2(t, 2.0) * 0.1;
    assert_eq!(float("wave").to_bits(), wave.to_bits());
    assert_eq!(float("power").to_bits(), libm::pow(1.7, t + 0.3).to_bits());
    assert_eq!(float("logged").to_bits(), libm::log(t + 1.5).to_bits());
    let turned =
        libm::cos(t) + libm::tan(t * 0.5) + libm::asin(0.3) + libm::acos(0.2) + libm::atan(t);
    assert_eq!(float("turned").to_bits(), turned.to_bits());
}
