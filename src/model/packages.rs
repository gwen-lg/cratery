/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

//! Data types for crate information and description, in addition to Cargo types

use chrono::NaiveDateTime;
use serde_derive::{Deserialize, Serialize};

use super::cargo::{CrateMetadata, IndexCrateMetadata, RegistryUser};

/// Gets the last info for a crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CrateInfo {
    /// The last metadata, if any
    pub(crate) metadata: Option<CrateMetadata>,
    /// Whether the entire package is deprecated
    #[serde(rename = "isDeprecated")]
    pub(crate) is_deprecated: bool,
    /// Whether versions of this crate can be completely removed, not simply yanked
    #[serde(rename = "canRemove")]
    pub(crate) can_remove: bool,
    /// Gets the versions in the index
    pub(crate) versions: Vec<CrateInfoVersion>,
    /// The build targets to use (for docs generation and deps analysis)
    pub(crate) targets: Vec<CrateInfoTarget>,
    /// The required capabilities for docs generation
    pub(crate) capabilities: Vec<String>,
}

/// A build targets to use (for docs generation and deps analysis)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CrateInfoTarget {
    /// The target triple
    pub(crate) target: String,
    /// Whether to require a native toolchain for this target
    #[serde(rename = "docsUseNative")]
    pub(crate) docs_use_native: bool,
}

/// The data for a crate version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CrateInfoVersion {
    /// The data from the index
    pub(crate) index: IndexCrateMetadata,
    /// The upload date time
    pub(crate) upload: NaiveDateTime,
    /// The user that uploaded the version
    #[serde(rename = "uploadedBy")]
    pub(crate) uploaded_by: RegistryUser,
    /// The number of times this version was downloaded
    #[serde(rename = "downloadCount")]
    pub(crate) download_count: i64,
    /// Gets the last time this crate version had its dependencies automatically checked
    #[serde(rename = "depsLastCheck")]
    pub(crate) deps_last_check: NaiveDateTime,
    /// Flag whether this crate has outdated dependencies
    #[serde(rename = "depsHasOutdated")]
    pub(crate) deps_has_outdated: bool,
    /// Flag whether CVEs have been filed against dependencies of this crate
    #[serde(rename = "depsHasCVEs")]
    pub(crate) deps_has_cves: bool,
    /// The documentation status
    pub(crate) docs: Vec<CrateInfoVersionDocs>,
}

/// The documentation status for a crate version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CrateInfoVersionDocs {
    /// The corresponding target
    pub(crate) target: String,
    /// Whether the documentation generation was attempted
    #[serde(rename = "isAttempted")]
    pub(crate) is_attempted: bool,
    /// Whether documentation was generated for this target
    #[serde(rename = "isPresent")]
    pub(crate) is_present: bool,
}
