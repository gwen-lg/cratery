/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

//! Data model

pub(crate) mod auth;
pub(crate) mod cargo;
pub(crate) mod config;
pub(crate) mod deps;
pub(crate) mod docs;
pub(crate) mod errors;
pub(crate) mod namegen;
pub(crate) mod osv;
pub(crate) mod packages;
pub(crate) mod stats;
pub(crate) mod worker;

use auth::TokenUsage;
use serde_derive::{Deserialize, Serialize};

/// The object representing the application version
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct AppVersion {
    /// The changeset that was used to build the app
    pub(crate) commit: String,
    /// The version tag, if any
    pub(crate) tag: String,
}

/// Information about the registry, as exposed on the web API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct RegistryInformation {
    /// The name to use for the registry in cargo and git config
    #[serde(rename = "registryName")]
    pub(crate) registry_name: String,
    /// The version of the locally installed toolchain
    #[serde(rename = "toolchainVersionStable")]
    pub(crate) toolchain_version_stable: semver::Version,
    /// The version of the locally installed toolchain
    #[serde(rename = "toolchainVersionNightly")]
    pub(crate) toolchain_version_nightly: semver::Version,
    /// The host target of the locally installed toolchain
    #[serde(rename = "toolchainHost")]
    pub(crate) toolchain_host: String,
    /// The known built-in targets in rustc
    #[serde(rename = "toolchainTargets")]
    pub(crate) toolchain_targets: Vec<String>,
}

/// A couple describing a crate with its name and the associated version
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct CrateVersion {
    /// The name of the crate
    pub(crate) package: String,
    /// The crate's version
    pub(crate) version: String,
}

/// An event can be handled asynchronously by the application
#[derive(Debug, Clone)]
pub(crate) enum AppEvent {
    /// The use of a token to authenticate
    TokenUse(TokenUsage),
    /// The download of a crate
    CrateDownload(CrateVersion),
}

/// The modifier for the stable channel
pub(crate) const CHANNEL_STABLE: &str = "+stable";

/// The modifier for the nightly channel
pub(crate) const CHANNEL_NIGHTLY: &str = "+nightly";
