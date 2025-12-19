use crate::MessageEnvelope;
use serde::{de::DeserializeOwned, Serialize};
use std::{env, fmt};

#[derive(Debug)]
pub enum MessagingError {
    Zmq(zmq::Error),
    Serde(serde_json::Error),
    InvalidFrame(String),
    Utf8(String),
}

impl fmt::Display for MessagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zmq(err) => write!(f, "zmq error: {}", err),
            Self::Serde(err) => write!(f, "serialization error: {}", err),
            Self::InvalidFrame(message) => write!(f, "invalid frame: {}", message),
            Self::Utf8(message) => write!(f, "utf8 error: {}", message),
        }
    }
}

impl std::error::Error for MessagingError {}

impl From<zmq::Error> for MessagingError {
    fn from(value: zmq::Error) -> Self {
        Self::Zmq(value)
    }
}

impl From<serde_json::Error> for MessagingError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

#[derive(Debug, Clone)]
pub struct ZmqPublisherConfig {
    pub endpoint: String,
    pub bind: bool,
    pub high_water_mark: Option<i32>,
    pub linger_ms: Option<i32>,
}

impl ZmqPublisherConfig {
    pub fn bind(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            bind: true,
            high_water_mark: None,
            linger_ms: Some(0),
        }
    }

    pub fn connect(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            bind: false,
            high_water_mark: None,
            linger_ms: Some(0),
        }
    }

    pub fn from_env(default_endpoint: &str) -> Self {
        let endpoint = env::var("C2_ZMQ_PUB_ENDPOINT")
            .unwrap_or_else(|_| default_endpoint.to_string());
        let bind = env_var_bool("C2_ZMQ_PUB_BIND", true);
        let high_water_mark = env_var_i32("C2_ZMQ_PUB_HWM");
        let linger_ms = env_var_i32("C2_ZMQ_PUB_LINGER_MS").or(Some(0));
        Self {
            endpoint,
            bind,
            high_water_mark,
            linger_ms,
        }
    }
}

pub struct ZmqPublisher {
    socket: zmq::Socket,
}

impl ZmqPublisher {
    pub fn new(config: &ZmqPublisherConfig) -> Result<Self, MessagingError> {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::PUB)?;
        if let Some(hwm) = config.high_water_mark {
            socket.set_sndhwm(hwm)?;
        }
        if let Some(linger) = config.linger_ms {
            socket.set_linger(linger)?;
        }
        if config.bind {
            socket.bind(&config.endpoint)?;
        } else {
            socket.connect(&config.endpoint)?;
        }
        Ok(Self { socket })
    }

    pub fn publish<T: Serialize>(
        &self,
        topic: &str,
        envelope: &MessageEnvelope<T>,
    ) -> Result<(), MessagingError> {
        let payload = serde_json::to_vec(envelope)?;
        self.socket.send_multipart([topic.as_bytes(), payload.as_slice()], 0)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ZmqSubscriberConfig {
    pub endpoint: String,
    pub bind: bool,
    pub topics: Vec<String>,
    pub high_water_mark: Option<i32>,
    pub linger_ms: Option<i32>,
}

impl ZmqSubscriberConfig {
    pub fn connect(endpoint: impl Into<String>, topics: Vec<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            bind: false,
            topics,
            high_water_mark: None,
            linger_ms: Some(0),
        }
    }

    pub fn bind(endpoint: impl Into<String>, topics: Vec<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            bind: true,
            topics,
            high_water_mark: None,
            linger_ms: Some(0),
        }
    }

    pub fn from_env(default_endpoint: &str) -> Self {
        let endpoint = env::var("C2_ZMQ_SUB_ENDPOINT")
            .unwrap_or_else(|_| default_endpoint.to_string());
        let bind = env_var_bool("C2_ZMQ_SUB_BIND", false);
        let topics = env::var("C2_ZMQ_SUB_TOPICS")
            .unwrap_or_default()
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        let high_water_mark = env_var_i32("C2_ZMQ_SUB_HWM");
        let linger_ms = env_var_i32("C2_ZMQ_SUB_LINGER_MS").or(Some(0));
        Self {
            endpoint,
            bind,
            topics,
            high_water_mark,
            linger_ms,
        }
    }
}

pub struct ZmqSubscriber {
    socket: zmq::Socket,
}

impl ZmqSubscriber {
    pub fn new(config: &ZmqSubscriberConfig) -> Result<Self, MessagingError> {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::SUB)?;
        if let Some(hwm) = config.high_water_mark {
            socket.set_rcvhwm(hwm)?;
        }
        if let Some(linger) = config.linger_ms {
            socket.set_linger(linger)?;
        }
        if config.bind {
            socket.bind(&config.endpoint)?;
        } else {
            socket.connect(&config.endpoint)?;
        }
        if config.topics.is_empty() {
            socket.set_subscribe(b"")?;
        } else {
            for topic in &config.topics {
                socket.set_subscribe(topic.as_bytes())?;
            }
        }
        Ok(Self { socket })
    }

    pub fn recv<T: DeserializeOwned>(&self) -> Result<(String, MessageEnvelope<T>), MessagingError> {
        let frames = self.socket.recv_multipart(0)?;
        if frames.len() < 2 {
            return Err(MessagingError::InvalidFrame(
                "expected topic and payload frames".to_string(),
            ));
        }
        let topic = String::from_utf8(frames[0].clone())
            .map_err(|_| MessagingError::Utf8("invalid topic utf8".to_string()))?;
        let envelope = serde_json::from_slice(&frames[1])?;
        Ok((topic, envelope))
    }
}

fn env_var_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        })
        .unwrap_or(default)
}

fn env_var_i32(key: &str) -> Option<i32> {
    env::var(key).ok().and_then(|value| value.parse::<i32>().ok())
}
