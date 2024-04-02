// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use aws_sdk_s3::config::Builder;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use aws_types::sdk_config::SdkConfig;

/// Creates a new client from an [SDK config](aws_types::sdk_config::SdkConfig)
/// with Materialize-specific customizations.
///
/// Specifically, if the SDK config overrides the endpoint URL, the client
/// will be configured to use path-style addressing, as custom AWS endpoints
/// typically do not support virtual host-style addressing.
pub fn new_client(sdk_config: &SdkConfig) -> Client {
    let conf = Builder::from(sdk_config)
        .force_path_style(sdk_config.endpoint_url().is_some())
        .build();
    Client::from_conf(conf)
}

pub async fn list_bucket_path(
    client: &Client,
    bucket: &str,
    prefix: &str,
) -> Result<Option<Vec<String>>, anyhow::Error> {
    let res = client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(prefix)
        .send()
        .await?;
    Ok(res
        .contents
        .map(|objs| {
            objs.into_iter()
                .map(|obj| {
                    obj.key
                        .ok_or(anyhow::anyhow!("key not provided from list_objects_v2"))
                })
                .collect::<Result<Vec<String>, _>>()
        })
        .transpose()?)
}

/// Upload an object to an S3 bucket. `mz_aws_util::s3_uploader::S3MultiPartUploader`
/// should be used instead for large files.
pub async fn upload_object<T: Into<ByteStream>>(
    client: &Client,
    bucket: &str,
    key: &str,
    body: T,
) -> Result<(), aws_sdk_s3::Error> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body.into())
        .send()
        .await?;
    Ok(())
}

pub async fn delete_object(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<(), aws_sdk_s3::Error> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    Ok(())
}
