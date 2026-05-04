mod cubemap {
    use glam::Vec3;
    use lightbaker::cubemap::{nearest_dir, parallax_corrected_dir, ProbeObb};

    #[test]
    fn parallax_landing() {
        let probe = ProbeObb {
            center: Vec3::new(0.0, 1.0, 0.0),
            half_extents: Vec3::new(2.0, 1.0, 2.0),
        };
        let target = Vec3::new(0.0, 2.0, 0.0);
        let s_a = Vec3::new(1.0, 0.0, 0.0);
        let s_b = Vec3::new(-1.0, 0.0, 0.0);
        let dir_a = (target - s_a).normalize();
        let dir_b = (target - s_b).normalize();

        let p_a = parallax_corrected_dir(&probe, s_a, dir_a);
        let p_b = parallax_corrected_dir(&probe, s_b, dir_b);
        let same = (p_a - p_b).abs().max_element();
        assert!(
            same < 0.05,
            "parallax-corrected directions diverge: {p_a:?} vs {p_b:?}",
        );

        let nearest_a = nearest_dir(dir_a);
        let nearest_b = nearest_dir(dir_b);
        let differ = (nearest_a - nearest_b).abs().max_element();
        assert!(
            differ > 0.5,
            "nearest-cubemap should land at different points: {nearest_a:?} vs {nearest_b:?}",
        );
    }
}
