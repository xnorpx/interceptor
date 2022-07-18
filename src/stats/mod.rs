use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

mod receiver;
mod sender;
#[cfg(test)]
mod stats_test;

#[derive(Clone, Debug, Default)]
struct RTPStats {
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

    pub fn packets(&self) -> u64 {
        self.packets.load(Ordering::SeqCst)
    }

    pub fn header_bytes(&self) -> u64 {
        self.header_bytes.load(Ordering::SeqCst)
    }

    pub fn payload_bytes(&self) -> u64 {
        self.payload_bytes.load(Ordering::SeqCst)
    }
}
