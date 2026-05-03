// stage 7 gate, the load-bearing test. un-ignore once the radiosity solver runs.
// reference: docs/testing.md and docs/algorithm-notes.md.
// closed unit cube, reflectivity rho=0.5, single emissive patch with B0=1.0.
// at convergence, total radiosity should equal 1/(1-rho) = 2.0 within 1%.

#[test]
#[ignore = "stage 7 not implemented"]
fn closed_box_within_1pct() {
    // let scene = lightbaker::test_scenes::closed_unit_cube(0.5);
    // let result = lightbaker::radiosity::solve(&scene, lightbaker::radiosity::Settings::default());
    // let total: f32 = result.patches.iter().map(|p| p.radiosity).sum();
    // let expected = 1.0 / (1.0 - 0.5);
    // let err = (total - expected).abs() / expected;
    // assert!(err < 0.01, "energy leak: total={total} expected={expected} err={err}");
    unimplemented!("wire lightbaker::radiosity::solve");
}
