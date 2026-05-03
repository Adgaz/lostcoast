// stage 8 gate. un-ignore once the rnm composer runs.
// reference: docs/testing.md, stage 8.
// flat plane lit by single overhead light. all three rnm lightmaps should be ~equal.
// each Bi has the same +Z component of 1/sqrt(3).

#[test]
#[ignore = "stage 8 not implemented"]
fn flat_plane_three_equal() {
    // let scene = lightbaker::test_scenes::flat_plane_overhead();
    // let baked = lightbaker::bake_rnm(&scene);
    // for texel in baked.texels() {
    //     let [b0, b1, b2] = texel.rnm;
    //     let max_diff = (b0 - b1).abs().max((b1 - b2).abs()).max((b0 - b2).abs());
    //     assert!(max_diff.max_element() < 1e-3, "rnm asymmetry: {b0:?} {b1:?} {b2:?}");
    // }
    unimplemented!("wire lightbaker::bake_rnm");
}
