// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, str::FromStr};

use semver::Version;
use serde::{de::Error, Deserialize, Deserializer};
use time::OffsetDateTime;
use url::Url;

use crate::{ReleaseManifestPlatform, RemoteRelease, RemoteReleaseInner, UpdateFormat};

fn parse_version<'de, D>(deserializer: D) -> std::result::Result<Version, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let str = String::deserialize(deserializer)?;
    Version::from_str(str.trim_start_matches('v')).map_err(Error::custom)
}

impl<'de> Deserialize<'de> for UpdateFormat {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let lower = String::deserialize(deserializer)?.to_lowercase();
        let variant = match lower.as_str() {
            "nsis" => UpdateFormat::Nsis,
            "wix" => UpdateFormat::Wix,
            "app" => UpdateFormat::App,
            "appimage" => UpdateFormat::AppImage,
            _ => {
                return Err(serde::de::Error::custom(
                    "Unkown updater format, expected one of 'nsis', 'wix', 'app' or 'appimage'",
                ))
            }
        };

        Ok(variant)
    }
}

impl<'de> Deserialize<'de> for RemoteRelease {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct InnerRemoteRelease {
            #[serde(alias = "name", deserialize_with = "parse_version")]
            version: Version,
            notes: Option<String>,
            pub_date: Option<String>,
            platforms: Option<HashMap<String, ReleaseManifestPlatform>>,
            // dynamic platform response
            url: Option<Url>,
            signature: Option<String>,
            format: Option<UpdateFormat>,
        }

        let release = InnerRemoteRelease::deserialize(deserializer)?;

        let pub_date = if let Some(date) = release.pub_date {
            Some(
                OffsetDateTime::parse(&date, &time::format_description::well_known::Rfc3339)
                    .map_err(|e| {
                        serde::de::Error::custom(format!("invalid value for `pub_date`: {e}"))
                    })?,
            )
        } else {
            None
        };

        Ok(RemoteRelease {
            version: release.version,
            notes: release.notes,
            pub_date,
            data: if let Some(platforms) = release.platforms {
                RemoteReleaseInner::Static { platforms }
            } else {
                RemoteReleaseInner::Dynamic(ReleaseManifestPlatform {
                    url: release.url.ok_or_else(|| {
                        Error::custom("the `url` field was not set on the updater response")
                    })?,
                    signature: release.signature.ok_or_else(|| {
                        Error::custom("the `signature` field was not set on the updater response")
                    })?,
                    format: release.format.ok_or_else(|| {
                        Error::custom("the `format` field was not set on the updater response")
                    })?,
                })
            },
        })
    }
}
