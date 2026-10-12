//! Kubernetes quantities and the gauge they make.

#[test]
fn a_quantity_reads_into_base_units_and_writes_back() {
    use crate::{bytes, cores, quantity};
    assert_eq!(quantity("250m"), Some(0.25));
    assert_eq!(quantity("1Gi"), Some(1_073_741_824.0));
    assert_eq!(quantity("512Mi"), Some(536_870_912.0));
    assert_eq!(quantity("2"), Some(2.0));
    let nano = quantity("123456789n").expect("nano cores");
    assert!((nano - 0.123_456_789).abs() < 1e-12);
    assert_eq!(quantity("1.5k"), Some(1500.0));
    assert_eq!(cores(0.18), "180m");
    assert_eq!(cores(2.0), "2");
    assert_eq!(bytes(734_003_200.0), "700Mi");
    assert_eq!(bytes(2_147_483_648.0), "2Gi");
    assert_eq!(
        bytes(1.6 * 1_073_741_824.0),
        "1638Mi",
        "never rounded to 2Gi"
    );
    assert_eq!(bytes(12.4 * 1_073_741_824.0), "12Gi");
}

#[test]
fn a_gauge_stands_on_the_limit_and_says_how_close_usage_comes() {
    use crate::{Pressure, gauge};
    let over = gauge(Some(0.5), Some(1.0), Some(0.7)).expect("a gauge");
    assert_eq!(
        (over.request, over.used, over.pressure),
        (Some(0.5), Some(0.7), Pressure::Over)
    );
    let near = gauge(Some(0.5), Some(1.0), Some(0.95)).expect("a gauge");
    assert_eq!(near.pressure, Pressure::Near);
    let unlimited = gauge(Some(0.4), None, Some(0.2)).expect("a gauge");
    assert_eq!(unlimited.request, Some(0.8));
    assert_eq!(gauge(None, None, None), None);
}
