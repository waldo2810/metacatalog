// Copyright 2026 The MetaCatalog Authors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! MetaCatalog (`mc`) — a data catalog whose source of truth is YAML under
//! `catalog/**`.
//!
//! The crate exposes a library target so that unit tests can run with
//! `cargo test --lib`, with no external services, no network and no Docker.

pub mod yaml;
