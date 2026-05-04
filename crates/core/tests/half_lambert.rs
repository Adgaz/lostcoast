mod half_lambert {
    use lostcoast_core::math::half_lambert_from_dot;

    #[test]
    fn table_matches() {
        let cases = [
            (-1.0_f32, 0.0_f32),
            (-0.5, 0.0625),
            (0.0, 0.25),
            (0.5, 0.5625),
            (1.0, 1.0),
        ];
        for (d, expected) in cases {
            let got = half_lambert_from_dot(d);
            assert!(
                (got - expected).abs() < 1e-6,
                "half_lambert({d}) = {got}, expected {expected}",
            );
        }
    }
}
