// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Implementation-specific metrics for persist blobs and consensus

use mz_ore::metric;
use mz_ore::metrics::{IntCounter, MetricsRegistry};
use prometheus::IntCounterVec;

/// Metrics specific to S3Blob's internal workings.
#[derive(Debug, Clone)]
pub struct S3BlobMetrics {
    pub(crate) operation_timeouts: IntCounter,
    pub(crate) operation_attempt_timeouts: IntCounter,
    pub(crate) connect_timeouts: IntCounter,
    pub(crate) read_timeouts: IntCounter,
    pub(crate) get_part: IntCounter,
    pub(crate) set_single: IntCounter,
    pub(crate) set_multi_create: IntCounter,
    pub(crate) set_multi_part: IntCounter,
    pub(crate) set_multi_complete: IntCounter,
    pub(crate) delete_head: IntCounter,
    pub(crate) delete_object: IntCounter,
    pub(crate) list_objects: IntCounter,
}

impl S3BlobMetrics {
    /// Returns a new [S3BlobMetrics] instance connected to the given registry.
    pub fn new(registry: &MetricsRegistry) -> Self {
        let operations: IntCounterVec = registry.register(metric!(
            name: "mz_persist_s3_operations",
            help: "number of raw s3 calls on behalf of Blob interface methods",
            var_labels: ["op"],
        ));
        Self {
            operation_timeouts: registry.register(metric!(
                name: "mz_persist_s3_operation_timeouts",
                help: "number of operation timeouts (including retries)",
            )),
            operation_attempt_timeouts: registry.register(metric!(
                name: "mz_persist_s3_operation_attempt_timeouts",
                help: "number of operation attempt timeouts (within a single retry)",
            )),
            connect_timeouts: registry.register(metric!(
                name: "mz_persist_s3_connect_timeouts",
                help: "number of timeouts establishing a connection to S3",
            )),
            read_timeouts: registry.register(metric!(
                name: "mz_persist_s3_read_timeouts",
                help: "number of timeouts waiting on first response byte from S3",
            )),
            get_part: operations.with_label_values(&["get_part"]),
            set_single: operations.with_label_values(&["set_single"]),
            set_multi_create: operations.with_label_values(&["set_multi_create"]),
            set_multi_part: operations.with_label_values(&["set_multi_part"]),
            set_multi_complete: operations.with_label_values(&["set_multi_complete"]),
            delete_head: operations.with_label_values(&["delete_head"]),
            delete_object: operations.with_label_values(&["delete_object"]),
            list_objects: operations.with_label_values(&["list_objects"]),
        }
    }
}
