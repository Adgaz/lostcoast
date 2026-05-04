mod hdr {
    use lightbaker::hdr::step_response_frames;

    #[test]
    fn histogram_converges_within_30_frames() {
        let initial = 0.1_f32;
        let target = 4.0_f32;
        let time_constant = 0.1_f32;
        let dt = 1.0 / 60.0;
        let tolerance = 0.1;
        let frames = step_response_frames(initial, target, time_constant, dt, tolerance);
        assert!(
            frames <= 30,
            "took {frames} frames to converge, expected <= 30"
        );
    }
}
