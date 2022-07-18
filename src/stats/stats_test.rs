use crate::stats::RTPStats;

#[test]
fn test_rtp_stats() {
    let stats: RTPStats = Default::default();
    assert_eq!(
        (stats.header_bytes(), stats.payload_bytes(), stats.packets()),
        (0, 0, 0),
    );

    stats.update(24, 960, 1);

    assert_eq!(
        (stats.header_bytes(), stats.payload_bytes(), stats.packets()),
        (24, 960, 1),
    );
}

#[test]
fn test_rtp_stats_send_sync() {
    const fn test_send_sync<T: Send + Sync>() {}
    test_send_sync::<RTPStats>();
}
