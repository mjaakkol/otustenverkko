//! Asyncronous MQTT with cloud hub support
//!
//! This crate implements standard MQTT with selected cloud hub
//! to make it easy to take into use.
//!

#![forbid(unsafe_code)]
#![allow(dead_code)]
//#![warn(missing_docs, rust_2021_idioms)]
#![warn(missing_docs)]

mod packets;
mod validation;

/// Asynchronous MQTT implementation.
///
/// This is one of the many asynchronous MQTT implementations build on top of smol-rs libraries.
/// It can be used 'as-is' but other libraries in the same crate are using this to implement cloud
/// cloud IoT hubs.
pub mod mqtt;

/// Google MQTT implementation.
///
/// Module build on top of MQTT to reduce the integration steps to bring device into
/// Google IoT Core platform. It performs the necessary authentication steps and subscribes
/// to the default topic automatically during connect. This makes adopting Google IoT
/// Core just a bit easier
pub mod gcp_mqtt;
