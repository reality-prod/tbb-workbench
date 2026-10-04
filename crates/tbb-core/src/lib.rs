//! `tbb-core`: all Tauri-independent logic for TBB Workbench.
//!
//! Keeping this separate from the Tauri shell means every safety-relevant
//! behaviour (path containment, process supervision, redaction, log storage,
//! diagnostics) is unit- and integration-testable without a webview.

pub mod artifacts;
pub mod build;
pub mod diagnostics;
pub mod discovery;
pub mod editor;
pub mod git;
pub mod logs;
pub mod models;
pub mod preflight;
pub mod rbm;
pub mod repository;
pub mod security;
pub mod settings;
pub mod system;
pub mod util;
