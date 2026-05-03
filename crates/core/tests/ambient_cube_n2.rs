// stage 9 gate. un-ignore once core::ambient_cube exists.
// reference: docs/testing.md, ambient cube n^2.
// cube with +Y white, other axes black.

#[test]
#[ignore = "stage 9 not implemented"]
fn n2_axes() {
    // axes: +x -x +y -y +z -z; only +y is white.
    // let cube = core::ambient_cube::AmbientCube::from_axes([
    //     [0.0; 3], [0.0; 3], [1.0; 3], [0.0; 3], [0.0; 3], [0.0; 3],
    // ]);
    //
    // let up = cube.sample(glam::Vec3::Y);
    // assert!((up - glam::Vec3::splat(1.0)).abs().max_element() < 1e-5);
    //
    // let right = cube.sample(glam::Vec3::X);
    // assert!(right.abs().max_element() < 1e-5);
    //
    // let diag = cube.sample(glam::Vec3::new(1.0, 1.0, 0.0).normalize());
    // assert!((diag - glam::Vec3::splat(0.5)).abs().max_element() < 1e-3);
    unimplemented!("wire core::ambient_cube::AmbientCube");
}
