// Packets
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use crate::mqtt::{MqttError, PacketFutureResult};
use log::{debug, /* info, error,*/ warn}; //,MqttError};

#[derive(Clone)]
pub(crate) struct PacketFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

pub(crate) struct SharedState {
    completed: Option<Result<PacketFutureResult, MqttError>>,
    waker: Option<Waker>,
}

impl PacketFuture {
    pub fn new() -> PacketFuture {
        PacketFuture {
            shared_state: Arc::new(Mutex::new(SharedState {
                completed: None,
                waker: None,
            })),
        }
    }

    pub fn complete_ok(self) {
        self.complete(Ok(PacketFutureResult::Ok));
    }

    pub fn complete_with_err(self, err: MqttError) {
        self.complete(Err(err));
    }

    pub fn complete(self, status: Result<PacketFutureResult, MqttError>) {
        debug!("PacketFuture complete started");

        let mut shared_state = self.shared_state.lock().unwrap();

        shared_state.completed = Some(status);

        if let Some(waker) = shared_state.waker.take() {
            debug!("Waker found and about to trigger");
            waker.wake();
        } else {
            // Not sure what can be done here. If the future gets polled again, it will be released
            // but this code is not really helping to do that. Should this crash the application?
            warn!("ConAck error: Taking waker failed");
        }
    }
}

impl Future for PacketFuture {
    type Output = Result<PacketFutureResult, MqttError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Look at the shared state to see if the timer has already completed.
        debug!("PacketFuture: Poll");
        let mut shared_state = self.shared_state.lock().unwrap();

        if let Some(result) = shared_state.completed.take() {
            debug!("PacketFuture: Future marked ready");
            Poll::Ready(result)
        } else {
            debug!("PacketFuture: Future pending");
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
