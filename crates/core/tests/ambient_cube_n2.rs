mod ambient_cube {
    use glam::Vec3;
    use lostcoast_core::ambient_cube::AmbientCube;

    #[test]
    fn n2_axes() {
        let cube =
            AmbientCube::from_axes([[0.0; 3], [0.0; 3], [1.0; 3], [0.0; 3], [0.0; 3], [0.0; 3]]);

        let up = cube.sample(Vec3::Y);
        assert!((up - Vec3::splat(1.0)).abs().max_element() < 1e-5);

        let right = cube.sample(Vec3::X);
        assert!(right.abs().max_element() < 1e-5);

        let diag = cube.sample(Vec3::new(1.0, 1.0, 0.0).normalize());
        assert!((diag - Vec3::splat(0.5)).abs().max_element() < 1e-3);
    }
}
