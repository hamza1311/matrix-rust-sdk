use serde::{Deserialize, Serialize};

pub mod from_widget;
mod message;
pub mod to_widget;

pub use self::message::{Empty, Kind as MessageKind, Request, Response};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "api")]
#[serde(rename_all = "camelCase")]
pub enum Action {
    FromWidget(from_widget::Action),
    ToWidget(to_widget::Action),
}
