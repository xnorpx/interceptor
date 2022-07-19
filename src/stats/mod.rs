use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

mod interceptor;

pub use interceptor::StatsInterceptor;

pub fn make_stats_interceptor(id: &str) -> Arc<StatsInterceptor> {
    Arc::new(StatsInterceptor::new(id.to_owned()))
}

#[derive(Debug, Default)]
/// Records stats about a given RTP stream.
pub struct RTPStats {
    /// Packets sent or received
    packets: Arc<AtomicU64>,

    /// Payload bytes sent or received
    payload_bytes: Arc<AtomicU64>,

    /// Header bytes sent or received
    header_bytes: Arc<AtomicU64>,
}

impl RTPStats {
    pub fn update(&self, header_bytes: u64, payload_bytes: u64, packets: u64) {
        self.header_bytes.fetch_add(header_bytes, Ordering::SeqCst);
        self.payload_bytes
            .fetch_add(payload_bytes, Ordering::SeqCst);
        self.packets.fetch_add(packets, Ordering::SeqCst);
    }

    pub fn reader(&self) -> RTPStatsReader {
        RTPStatsReader {
            packets: self.packets.clone(),
            payload_bytes: self.payload_bytes.clone(),
            header_bytes: self.header_bytes.clone(),
        }
    }
}

#[derive(Clone, Debug, Default)]
/// Reader half of RTPStats.
pub struct RTPStatsReader {
    packets: Arc<AtomicU64>,
    payload_bytes: Arc<AtomicU64>,
    header_bytes: Arc<AtomicU64>,
}

impl RTPStatsReader {
    /// Get packets sent or received.
    pub fn packets(&self) -> u64 {
        self.packets.load(Ordering::SeqCst)
    }

    /// Get payload bytes sent or received.
    pub fn header_bytes(&self) -> u64 {
        self.header_bytes.load(Ordering::SeqCst)
    }

    /// Get header bytes sent or received.
    pub fn payload_bytes(&self) -> u64 {
        self.payload_bytes.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rtp_stats() {
        let stats: RTPStats = Default::default();
        let reader = stats.reader();
        assert_eq!(
            (
                reader.header_bytes(),
                reader.payload_bytes(),
                reader.packets()
            ),
            (0, 0, 0),
        );

        stats.update(24, 960, 1);

        assert_eq!(
            (
                reader.header_bytes(),
                reader.payload_bytes(),
                reader.packets()
            ),
            (24, 960, 1),
        );
    }

    #[test]
    fn test_rtp_stats_send_sync() {
        fn test_send_sync<T: Send + Sync>() {}
        test_send_sync::<RTPStats>();
    }
}
