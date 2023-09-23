#![warn(clippy::all, rust_2018_idioms)]

extern crate self as kaspa_ng_core;

mod wallet;
pub use wallet::Wallet;

pub mod adaptor;
pub mod egui;
pub mod error;
pub mod events;
// pub mod icons;
pub mod imports;
pub mod interop;
pub mod modules;
pub mod network;
pub mod notifications;
pub mod panel;
pub mod primitives;
pub mod prompt;
pub mod result;
pub mod settings;
pub mod sync;
// pub mod theme;
pub mod utils;

// #[cfg(any(feature = "browser-extension", feature="desktop-application"))]
pub mod app;