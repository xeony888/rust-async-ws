pub fn clamp_f64(float: f64) -> f64 {
    if float > -0.1 && float < 0.1 {
        return 0.0;
    } else {
        return float;
    }
}
