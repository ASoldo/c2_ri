use c2_core::{CorrelationId, EpochMillis, MessageId, SecurityClassification, TenantId};
use serde::{Deserialize, Serialize};

mod zmq_transport;
pub use zmq_transport::{
    MessagingError, ZmqPublisher, ZmqPublisherConfig, ZmqSubscriber, ZmqSubscriberConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub message_id: MessageId,
    pub correlation_id: Option<CorrelationId>,
    pub tenant_id: TenantId,
    pub classification: SecurityClassification,
    pub sent_at_ms: EpochMillis,
    pub source_service: String,
    pub destination: Option<String>,
    pub schema: Option<String>,
}

impl MessageMetadata {
    pub fn new(
        message_id: MessageId,
        tenant_id: TenantId,
        classification: SecurityClassification,
        sent_at_ms: EpochMillis,
        source_service: String,
    ) -> Self {
        Self {
            message_id,
            correlation_id: None,
            tenant_id,
            classification,
            sent_at_ms,
            source_service,
            destination: None,
            schema: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    pub metadata: MessageMetadata,
    pub payload: T,
}
