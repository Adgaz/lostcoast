// stage 5 gate. un-ignore once core::math::half_lambert exists.
// reference: docs/testing.md, half-lambert table.
//
// expected: ((dot * 0.5) + 0.5)^2
//   dot=-1.0   -> 0.0
//   dot=-0.5   -> 0.0625
//   dot= 0.0   -> 0.25
//   dot= 0.5   -> 0.5625
//   dot= 1.0   -> 1.0

#[test]
#[ignore = "stage 5 not implemented"]
fn table_matches() {
    unimplemented!("wire core::math::half_lambert_from_dot");
}
