//! # fpj — File Projector
//!
//! `fpj` is an application-agnostic tool for creating and managing flexible
//! filesystem views using layered overlays and bind mounts.
//!
//! It lets you define **layers** (overlay filesystems with writable/locked
//! states) and **layouts** (ordered sequences of mount operations) that can be
//! mounted, unmounted, and restored atomically. All state is persisted in a
//! local SQLite database.
//!
//! ## Core concepts
//!
//! - [`model::Layer`] — a named overlay layer backed by a directory or another
//!   locked layer.
//! - [`model::Layout`] — an ordered list of [`model::MountStepDef`] entries
//!   (overlay layers and bind mounts).
//! - [`engine::LayoutEngine`] — the main entry point that ties layers, layouts,
//!   and mount operations together.
//! - [`backend::MountBackend`] — platform-specific trait implemented for Linux,
//!   macOS, and Windows.
//!
//! ## Quick example
//!
//! ```no_run
//! use fpj::database::LayoutDatabase;
//! use fpj::engine::LayoutEngine;
//! use fpj::backend::create_backend;
//! use fpj::model::LayerSource;
//! use std::path::PathBuf;
//!
//! let db = LayoutDatabase::open(&fpj::engine::default_db_path()).unwrap();
//! let engine = LayoutEngine::new(db, create_backend());
//!
//! // Create a layer from an existing directory
//! engine.create_layer(
//!     "base",
//!     LayerSource::Directory(PathBuf::from("/opt/platform")),
//!     PathBuf::from("/workspace/merged"),
//! ).unwrap();
//! ```

pub mod backend;
pub mod database;
pub mod engine;
pub mod error;
pub mod model;
pub mod operations;
