use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use super::RTPStats;
use async_trait::async_trait;
use util::sync::Mutex;
use util::Unmarshal;

use crate::error::Result;
use crate::stream_info::StreamInfo;
use crate::{
    Attributes, Interceptor, InterceptorBuilder, RTCPReader, RTCPWriter, RTPReader, RTPWriter,
};

#[derive(Debug)]
pub struct RecieverBuilder {}

impl InterceptorBuilder for RecieverBuilder {
    fn build(&self, id: &str) -> Result<Arc<dyn crate::Interceptor + Send + Sync>> {
        Ok(Arc::new(Receiver::new(id.to_owned())))
    }
}

pub struct RTPRecorder {
    rtp_reader: Arc<dyn RTPReader + Send + Sync>,
    stats: RTPStats,
}

impl RTPRecorder {
    fn new(rtp_reader: Arc<dyn RTPReader + Send + Sync>) -> Self {
        Self {
            rtp_reader,
            stats: Default::default(),
        }
    }
}

#[async_trait]
impl RTPReader for RTPRecorder {
    async fn read(&self, buf: &mut [u8], attributes: &Attributes) -> Result<(usize, Attributes)> {
        let (bytes_read, attributes) = self.rtp_reader.read(buf, attributes).await?;
        // TODO: This parsing happens redundantly in several interceptors, would be good if we
        // could not do this.
        let mut b = &buf[..bytes_read];
        let packet = rtp::packet::Packet::unmarshal(&mut b)?;

        self.stats.update(
            (bytes_read - packet.payload.len()) as u64,
            packet.payload.len() as u64,
            1,
        );

        Ok((bytes_read, attributes))
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
pub struct Receiver {
    streams: Mutex<HashMap<u32, Arc<RTPRecorder>>>,
    id: String,
}

impl Receiver {
    fn new(id: String) -> Self {
        Self {
            id,
            streams: Default::default(),
        }
    }
}

#[async_trait]
impl Interceptor for Receiver {
    /// bind_rtcp_reader lets you modify any incoming RTCP packets. It is called once per sender/receiver, however this might
    /// change in the future. The returned method will be called once per packet batch.
    async fn bind_rtcp_reader(
        &self,
        reader: Arc<dyn RTCPReader + Send + Sync>,
    ) -> Arc<dyn RTCPReader + Send + Sync> {
        reader
    }

    /// bind_remote_stream lets you modify any incoming RTP packets. It is called once for per RemoteStream. The returned method
    /// will be called once per rtp packet.
    async fn bind_remote_stream(
        &self,
        info: &StreamInfo,
        reader: Arc<dyn RTPReader + Send + Sync>,
    ) -> Arc<dyn RTPReader + Send + Sync> {
        let mut lock = self.streams.lock();

        let e = lock
            .entry(info.ssrc)
            .or_insert_with(|| Arc::new(RTPRecorder::new(reader)));

        e.clone()
    }

    /// unbind_remote_stream is called when the Stream is removed. It can be used to clean up any data related to that track.
    async fn unbind_remote_stream(&self, info: &StreamInfo) {
        let mut lock = self.streams.lock();

        lock.remove(&info.ssrc);
    }

    async fn close(&self) -> Result<()> {
        Ok(())
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

    /// bind_local_stream lets you modify any outgoing RTP packets. It is called once for per LocalStream. The returned method
    /// will be called once per rtp packet.
    async fn bind_local_stream(
        &self,
        _info: &StreamInfo,
        writer: Arc<dyn RTPWriter + Send + Sync>,
    ) -> Arc<dyn RTPWriter + Send + Sync> {
        // NOP
        writer
    }

    /// unbind_local_stream is called when the Stream is removed. It can be used to clean up any data related to that track.
    async fn unbind_local_stream(&self, _info: &StreamInfo) {}
}

#[cfg(test)]
mod test {
    use bytes::Bytes;

    use std::sync::Arc;

    use crate::error::Result;
    use crate::mock::mock_stream::MockStream;
    use crate::stream_info::StreamInfo;

    use super::Receiver;

    #[tokio::test]
    async fn test_receiver() -> Result<()> {
        let receiver: Arc<_> = Arc::new(Receiver::new("Hello".to_owned()));

        let stream = MockStream::new(
            &StreamInfo {
                ssrc: 123456,
                ..Default::default()
            },
            receiver.clone(),
        )
        .await;

        let recorder = receiver
            .streams
            .lock()
            .get(&123456)
            .cloned()
            .expect("A stream for SSRC 123456 should exist");

        let _ = stream
            .receive_rtp(rtp::packet::Packet {
                header: rtp::header::Header {
                    ..Default::default()
                },
                payload: Bytes::from_static(b"\xde\xad\xbe\xef"),
            })
            .await;

        let _ = stream
            .read_rtp()
            .await
            .expect("After calling receive_rtp read_rtp should return Some")?;

        assert_eq!(recorder.stats.packets(), 1);
        assert_eq!(recorder.stats.header_bytes(), 12);
        assert_eq!(recorder.stats.payload_bytes(), 4);

        Ok(())
    }
}
