mod energy {
    use lightbaker::radiosity::{solve, total_flux, Settings};
    use lightbaker::scene::BakeScene;

    #[test]
    fn closed_box_within_1pct() {
        let rho = 0.5_f32;
        let scene = BakeScene::closed_unit_cube(rho);
        let solution = solve(
            &scene,
            Settings {
                samples_per_patch: 8192,
                max_iters: 256,
                seed: 42,
                epsilon: 1e-5,
            },
        );
        let total = total_flux(&scene, &solution);
        let avg = (total.x + total.y + total.z) / 3.0;
        let expected = 1.0_f32 / (1.0 - rho);
        let err = (avg - expected).abs() / expected;
        assert!(
            err < 0.01,
            "energy leak: total={avg} expected={expected} err={err} iters={}",
            solution.iterations
        );
    }
}
