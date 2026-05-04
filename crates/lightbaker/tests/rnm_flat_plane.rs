mod rnm {
    use glam::Vec3;
    use lightbaker::rnm::flat_plane_overhead_texel;

    #[test]
    fn flat_plane_three_equal() {
        let texel = flat_plane_overhead_texel();
        let max_diff = (texel.rnm[0] - texel.rnm[1])
            .abs()
            .max((texel.rnm[1] - texel.rnm[2]).abs())
            .max((texel.rnm[0] - texel.rnm[2]).abs())
            .max_element();
        assert!(
            max_diff < 1e-3,
            "rnm asymmetry: {:?} {:?} {:?}",
            texel.rnm[0],
            texel.rnm[1],
            texel.rnm[2]
        );
        let z = 1.0 / 3.0_f32.sqrt();
        let expected = Vec3::splat(z);
        for (idx, lm) in texel.rnm.iter().enumerate() {
            assert!(
                (*lm - expected).abs().max_element() < 1e-3,
                "lm{idx} = {lm:?}, expected {expected:?}",
            );
        }
    }
}
