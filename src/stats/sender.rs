use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use super::RTPStats;
use async_trait::async_trait;
use util::sync::Mutex;
use util::MarshalSize;

use crate::error::Result;
use crate::stream_info::StreamInfo;
use crate::{
    Attributes, Interceptor, InterceptorBuilder, RTCPReader, RTCPWriter, RTPReader, RTPWriter,
};

#[derive(Debug, Default)]
pub struct Builder {}

impl InterceptorBuilder for Builder {
    fn build(&self, id: &str) -> Result<Arc<dyn crate::Interceptor + Send + Sync>> {
        Ok(Arc::new(Sender::new(id.to_owned())))
    }
}

pub struct RTPRecorder {
    rtp_writer: Arc<dyn RTPWriter + Send + Sync>,
    stats: RTPStats,
}

impl RTPRecorder {
    fn new(rtp_writer: Arc<dyn RTPWriter + Send + Sync>) -> Self {
        Self {
            rtp_writer,
            stats: Default::default(),
        }
    }
}

#[async_trait]
impl RTPWriter for RTPRecorder {
    /// write a rtp packet
    async fn write(&self, pkt: &rtp::packet::Packet, attributes: &Attributes) -> Result<usize> {
        let n = self.rtp_writer.write(pkt, attributes).await?;

        self.stats.update(
            pkt.header.marshal_size() as u64,
            pkt.payload.len() as u64,
            1,
        );

        Ok(n)
    }
}

impl fmt::Debug for RTPRecorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RTPRecorder")
            .field("stats", &self.stats)
            .finish()
    }
}

#[derive(Debug)]
pub struct Sender {
    streams: Mutex<HashMap<u32, Arc<RTPRecorder>>>,
    id: String,
}

impl Sender {
    fn new(id: String) -> Self {
        Self {
            id,
            streams: Default::default(),
        }
    }
}

#[async_trait]
impl Interceptor for Sender {
    async fn close(&self) -> Result<()> {
        Ok(())
    }

    /// bind_local_stream lets you modify any outgoing RTP packets. It is called once for per LocalStream. The returned method
    /// will be called once per rtp packet.
    async fn bind_local_stream(
        &self,
        info: &StreamInfo,
        writer: Arc<dyn RTPWriter + Send + Sync>,
    ) -> Arc<dyn RTPWriter + Send + Sync> {
        let mut lock = self.streams.lock();

        let e = lock
            .entry(info.ssrc)
            .or_insert_with(|| Arc::new(RTPRecorder::new(writer)));

        e.clone()
    }

    /// unbind_local_stream is called when the Stream is removed. It can be used to clean up any data related to that track.
    async fn unbind_local_stream(&self, info: &StreamInfo) {
        let mut lock = self.streams.lock();

        lock.remove(&info.ssrc);
    }

    /// bind_rtcp_reader lets you modify any incoming RTCP packets. It is called once per sender/receiver, however this might
    /// change in the future. The returned method will be called once per packet batch.
    async fn bind_rtcp_reader(
        &self,
        reader: Arc<dyn RTCPReader + Send + Sync>,
    ) -> Arc<dyn RTCPReader + Send + Sync> {
        reader
    }

    /// bind_rtcp_writer lets you modify any outgoing RTCP packets. It is called once per PeerConnection. The returned method
    /// will be called once per packet batch.
    async fn bind_rtcp_writer(
        &self,
        writer: Arc<dyn RTCPWriter + Send + Sync>,
    ) -> Arc<dyn RTCPWriter + Send + Sync> {
        // NOP
        writer
    }

    /// bind_remote_stream lets you modify any incoming RTP packets. It is called once for per RemoteStream. The returned method
    /// will be called once per rtp packet.
    async fn bind_remote_stream(
        &self,
        _info: &StreamInfo,
        reader: Arc<dyn RTPReader + Send + Sync>,
    ) -> Arc<dyn RTPReader + Send + Sync> {
        // NOP
        reader
    }

    /// unbind_remote_stream is called when the Stream is removed. It can be used to clean up any data related to that track.
    async fn unbind_remote_stream(&self, _info: &StreamInfo) {
        // NOP
    }
}

#[cfg(test)]
mod test {
    use bytes::Bytes;

    use std::sync::Arc;

    use crate::error::Result;
    use crate::mock::mock_stream::MockStream;
    use crate::stream_info::StreamInfo;

    use super::Sender;

    #[tokio::test]
    async fn test_sender() -> Result<()> {
        let sender: Arc<_> = Arc::new(Sender::new("Hello".to_owned()));

        let stream = MockStream::new(
            &StreamInfo {
                ssrc: 123456,
                ..Default::default()
            },
            sender.clone(),
        )
        .await;

        let recorder = sender
            .streams
            .lock()
            .get(&123456)
            .cloned()
            .expect("A stream for SSRC 123456 should exist");

        let _ = stream
            .write_rtp(&rtp::packet::Packet {
                header: rtp::header::Header {
                    ..Default::default()
                },
                payload: Bytes::from_static(b"\xde\xad\xbe\xef"),
            })
            .await?;

        assert_eq!(recorder.stats.packets(), 1);
        assert_eq!(recorder.stats.header_bytes(), 12);
        assert_eq!(recorder.stats.payload_bytes(), 4);

        Ok(())
    }
}
