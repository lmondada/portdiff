//! A crux logging capability

use crux_core::{
    capability::{CapabilityContext, Operation},
    macros::Capability,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum LogOperation {
    Info(String),
    Error(String),
}

impl Operation for LogOperation {
    type Output = ();
}

#[derive(Capability)]
pub struct LogCapability<Event> {
    context: CapabilityContext<LogOperation, Event>,
}

impl<Event: 'static> LogCapability<Event> {
    pub fn new(context: CapabilityContext<LogOperation, Event>) -> Self {
        Self { context }
    }

    /// Log an info message
    pub fn info(&self, message: String) {
        self.send_msg(LogOperation::Info(message));
    }

    /// Log an error message
    pub fn error(&self, message: String) {
        self.send_msg(LogOperation::Error(message));
    }

    fn send_msg(&self, msg: LogOperation) {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            ctx.notify_shell(msg).await;
        });
    }
}
