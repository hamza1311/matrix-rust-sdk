//! Types and traits related to the permissions that a widget can request from a
//! client.

use std::fmt;

use async_trait::async_trait;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use super::filter::{EventFilter, MessageLikeEventFilter, StateEventFilter};

const SEND_EVENT: &str = "org.matrix.msc2762.m.send.event";
const READ_EVENT: &str = "org.matrix.msc2762.m.receive.event";
const SEND_STATE: &str = "org.matrix.msc2762.m.send.state_event";
const READ_STATE: &str = "org.matrix.msc2762.m.receive.state_event";
const REQUIRES_CLIENT: &str = "io.element.requires_client";

/// Must be implemented by a component that provides functionality of deciding
/// whether a widget is allowed to use certain capabilities (typically by
/// providing a prompt to the user).
#[async_trait]
pub trait PermissionsProvider: Send + Sync + 'static {
    /// Receives a request for given permissions and returns the actual
    /// permissions that the clients grants to a given widget (usually by
    /// prompting the user).
    async fn acquire_permissions(&self, permissions: Permissions) -> Permissions;
}

/// Permissions that a widget can request from a client.
#[derive(Debug, Clone, Default)]
pub struct Permissions {
    /// Types of the messages that a widget wants to be able to fetch.
    pub read: Vec<EventFilter>,
    /// Types of the messages that a widget wants to be able to send.
    pub send: Vec<EventFilter>,
    /// If a widget requests this capability the client is not allowed
    /// to open the widget in a seperated browser.
    pub requires_client: bool,
}

struct PrintEventFilter<'a>(&'a EventFilter);

impl fmt::Display for PrintEventFilter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            EventFilter::MessageLike(filter) => PrintMessageLikeEventFilter(filter).fmt(f),
            EventFilter::State(filter) => PrintStateEventFilter(filter).fmt(f),
        }
    }
}

struct PrintMessageLikeEventFilter<'a>(&'a MessageLikeEventFilter);

impl fmt::Display for PrintMessageLikeEventFilter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            MessageLikeEventFilter::WithType(event_type) => {
                // TODO: escape `#` as `\#` and `\` as `\\` in event_type
                write!(f, "{event_type}")
            }
            MessageLikeEventFilter::RoomMessageWithMsgtype(msgtype) => {
                write!(f, "m.room.message#{msgtype}")
            }
        }
    }
}

struct PrintStateEventFilter<'a>(&'a StateEventFilter);

impl fmt::Display for PrintStateEventFilter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: escape `#` as `\#` and `\` as `\\` in event_type
        match self.0 {
            StateEventFilter::WithType(event_type) => write!(f, "{event_type}"),
            StateEventFilter::WithTypeAndStateKey(event_type, state_key) => {
                write!(f, "{event_type}#{state_key}")
            }
        }
    }
}

impl Serialize for Permissions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let seq_len = self.requires_client as usize + self.read.len() + self.send.len();
        let mut seq = serializer.serialize_seq(Some(seq_len))?;

        if self.requires_client {
            seq.serialize_element(REQUIRES_CLIENT)?;
        }
        for filter in &self.read {
            let name = match filter {
                EventFilter::MessageLike(_) => READ_EVENT,
                EventFilter::State(_) => READ_STATE,
            };
            seq.serialize_element(&format!("{name}:{}", PrintEventFilter(filter)))?;
        }
        for filter in &self.send {
            let name = match filter {
                EventFilter::MessageLike(_) => SEND_EVENT,
                EventFilter::State(_) => SEND_STATE,
            };
            seq.serialize_element(&format!("{name}:{}", PrintEventFilter(filter)))?;
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Permission {
            RequiresClient,
            Read(EventFilter),
            Send(EventFilter),
            Unknown,
        }

        impl<'de> Deserialize<'de> for Permission {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = ruma::serde::deserialize_cow_str(deserializer)?;
                if s == REQUIRES_CLIENT {
                    return Ok(Self::RequiresClient);
                }

                match s.split_once(':') {
                    Some((READ_EVENT, filter_s)) => Ok(Permission::Read(EventFilter::MessageLike(
                        parse_message_event_filter(filter_s),
                    ))),
                    Some((SEND_EVENT, filter_s)) => Ok(Permission::Send(EventFilter::MessageLike(
                        parse_message_event_filter(filter_s),
                    ))),
                    Some((READ_STATE, filter_s)) => {
                        Ok(Permission::Read(EventFilter::State(parse_state_event_filter(filter_s))))
                    }
                    Some((SEND_STATE, filter_s)) => {
                        Ok(Permission::Send(EventFilter::State(parse_state_event_filter(filter_s))))
                    }
                    _ => Ok(Self::Unknown),
                }
            }
        }

        fn parse_message_event_filter(s: &str) -> MessageLikeEventFilter {
            match s.strip_prefix("m.room.message#") {
                Some(msgtype) => MessageLikeEventFilter::RoomMessageWithMsgtype(msgtype.to_owned()),
                // TODO: Replace `\\` by `\` and `\#` by `#`, enforce no unescaped `#`
                None => MessageLikeEventFilter::WithType(s.into()),
            }
        }

        fn parse_state_event_filter(s: &str) -> StateEventFilter {
            // TODO: Search for un-escaped `#` only, replace `\\` by `\` and `\#` by `#`
            match s.split_once('#') {
                Some((event_type, state_key)) => {
                    StateEventFilter::WithTypeAndStateKey(event_type.into(), state_key.to_owned())
                }
                None => StateEventFilter::WithType(s.into()),
            }
        }

        let mut permissions = Permissions::default();
        for permission in Vec::<Permission>::deserialize(deserializer)? {
            match permission {
                Permission::RequiresClient => permissions.requires_client = true,
                Permission::Read(filter) => permissions.read.push(filter),
                Permission::Send(filter) => permissions.send.push(filter),
                // ignore unknown permissions
                Permission::Unknown => {}
            }
        }

        Ok(permissions)
    }
}
