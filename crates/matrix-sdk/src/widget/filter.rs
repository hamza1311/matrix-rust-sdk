use ruma::events::{MessageLikeEventType, StateEventType, TimelineEventType};
use serde::Deserialize;

use super::messages::from_widget::SendEventRequest;

/// Different kinds of filters for timeline events.
#[derive(Clone, Debug)]
pub enum EventFilter {
    /// Filter for message-like events.
    MessageLike(MessageLikeEventFilter),
    /// Filter for state events.
    State(StateEventFilter),
}

impl EventFilter {
    pub(super) fn matches(&self, matrix_event: &MatrixEventFilterInput) -> bool {
        match self {
            EventFilter::MessageLike(message_filter) => message_filter.matches(matrix_event),
            EventFilter::State(state_filter) => state_filter.matches(matrix_event),
        }
    }
}

/// Filter for message-like events.
#[derive(Clone, Debug)]
pub enum MessageLikeEventFilter {
    /// Matches message-like events with the given `type`.
    WithType(MessageLikeEventType),
    /// Matches `m.room.message` events with the given `msgtype`.
    RoomMessageWithMsgtype(String),
}

impl MessageLikeEventFilter {
    fn matches(&self, matrix_event: &MatrixEventFilterInput) -> bool {
        if matrix_event.state_key.is_some() {
            // State event doesn't match a message-like event filter.
            return false;
        }

        match self {
            MessageLikeEventFilter::WithType(event_type) => {
                matrix_event.event_type == TimelineEventType::from(event_type.clone())
            }
            MessageLikeEventFilter::RoomMessageWithMsgtype(msgtype) => {
                matrix_event.event_type == TimelineEventType::RoomMessage
                    && matrix_event.content.msgtype.as_ref() == Some(msgtype)
            }
        }
    }
}

/// Filter for state events.
#[derive(Clone, Debug)]
pub enum StateEventFilter {
    /// Matches state events with the given `type`, regardless of `state_key`.
    WithType(StateEventType),
    /// Matches state events with the given `type` and `state_key`.
    WithTypeAndStateKey(StateEventType, String),
}

impl StateEventFilter {
    fn matches(&self, matrix_event: &MatrixEventFilterInput) -> bool {
        let Some(state_key) = &matrix_event.state_key else {
            // Message-like event doesn't match a state event filter.
            return false;
        };

        match self {
            StateEventFilter::WithType(event_type) => {
                matrix_event.event_type == TimelineEventType::from(event_type.clone())
            }
            StateEventFilter::WithTypeAndStateKey(event_type, filter_state_key) => {
                matrix_event.event_type == TimelineEventType::from(event_type.clone())
                    && state_key == filter_state_key
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct MatrixEventFilterInput {
    #[serde(rename = "type")]
    pub(super) event_type: TimelineEventType,
    pub(super) state_key: Option<String>,
    pub(super) content: MatrixEventContent,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct MatrixEventContent {
    pub(super) msgtype: Option<String>,
}

impl MatrixEventFilterInput {
    pub(super) fn from_send_event_request(req: SendEventRequest) -> Self {
        let SendEventRequest { event_type, state_key, content } = req;
        Self {
            event_type,
            state_key,
            // If content fails to deserialize (msgtype is not a string),
            // pretend that there is no msgtype as far as filters are concerned
            content: serde_json::from_value(content).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: Write tests for EventFilter::matches
}
