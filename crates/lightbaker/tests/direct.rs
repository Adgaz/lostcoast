mod direct {
    use glam::Vec3;
    use lightbaker::direct::{floor_below_light, PointLight};
    use lightbaker::scene::BakeScene;

    #[test]
    fn point_light_floor() {
        let scene = BakeScene::cornell_one_light();
        let height = 1.5_f32;
        let light = PointLight {
            position: Vec3::new(0.0, height, 0.0),
            intensity: Vec3::splat(10.0),
        };
        let sample = Vec3::new(0.0, 1e-3, 0.0);
        let r = floor_below_light(&scene, &light, sample);
        let dist = height - sample.y;
        let expected_scalar = 10.0_f32 * (1.0_f32 / (std::f32::consts::PI * dist * dist));
        let expected = Vec3::splat(expected_scalar);
        let err = (r - expected).abs().max_element() / expected_scalar;
        assert!(
            err < 0.05,
            "direct lighting wrong: got {r:?} expected {expected:?}",
        );
    }
}
