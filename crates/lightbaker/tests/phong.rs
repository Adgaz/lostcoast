mod phong {
    use glam::Vec3;
    use lightbaker::phong::{reflect, specular_align};

    #[test]
    fn specular_aligns() {
        let n = Vec3::Y;
        let l = Vec3::new(1.0, 1.0, 0.0).normalize();
        let r = reflect(l, n);
        let v_aligned = r;
        let v_off = Vec3::new(-1.0, 1.0, 0.0).normalize();

        let aligned = specular_align(l, v_aligned, n);
        let off = specular_align(l, v_off, n);
        assert!(aligned > 0.99, "aligned R.V should be ~1, got {aligned}",);
        assert!(
            off < aligned,
            "off-axis R.V should be smaller, got off={off} aligned={aligned}",
        );
    }
}
