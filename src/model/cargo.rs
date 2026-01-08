/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

//! Data model for the Cargo web API

use std::collections::HashMap;
use std::io::Cursor;
use std::str::FromStr;

use byteorder::{LittleEndian, ReadBytesExt};
use serde_derive::{Deserialize, Serialize};

use crate::utils::apierror::{ApiError, error_invalid_request, specialize};
use crate::utils::hashes::sha256;

/// A crate to appear in search results
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct SearchResultCrate {
    /// Name of the crate
    pub(crate) name: String,
    /// The highest version available
    pub(crate) max_version: String,
    /// Whether the entire package is deprecated
    #[serde(rename = "isDeprecated")]
    pub(crate) is_deprecated: bool,
    /// Textual description of the crate
    pub(crate) description: String,
}

/// The metadata of the search results
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct SearchResultsMeta {
    /// Total number of results available on the server
    pub(crate) total: usize,
}

/// The search results for crates
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct SearchResults {
    /// The crates
    pub(crate) crates: Vec<SearchResultCrate>,
    /// The metadata
    pub(crate) meta: SearchResultsMeta,
}

/// A set of errors as a response for the web API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ApiResponseErrors {
    /// The individual errors
    pub(crate) errors: Vec<ApiResponseError>,
}

/// An error response for the web API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ApiResponseError {
    /// The details for the error
    pub(crate) detail: String,
}

impl From<ApiError> for ApiResponseErrors {
    fn from(err: ApiError) -> Self {
        Self {
            errors: vec![ApiResponseError { detail: err.to_string() }],
        }
    }
}

/// The result for a yank operation
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct YesNoResult {
    /// The value for the result
    pub(crate) ok: bool,
}

impl YesNoResult {
    /// Creates a new instance
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self { ok: true }
    }
}

/// The result for a yank operation
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct YesNoMsgResult {
    /// The value for the result
    pub(crate) ok: bool,
    /// A string message that will be displayed
    pub(crate) msg: String,
}

impl YesNoMsgResult {
    /// Creates a new instance
    #[must_use]
    pub(crate) const fn new(msg: String) -> Self {
        Self { ok: true, msg }
    }
}

/// The result when querying for owners
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct OwnersQueryResult {
    /// The list of owners
    pub(crate) users: Vec<RegistryUser>,
}

/// The query for adding/removing owners to a crate
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct OwnersChangeQuery {
    /// The login of the users
    pub(crate) users: Vec<String>,
}

/// A user for the registry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct RegistryUser {
    /// The unique identifier
    /// Expected for Cargo
    pub(crate) id: i64,
    /// Whether this is an active user
    #[serde(rename = "isActive")]
    pub(crate) is_active: bool,
    /// The email, unique for each user
    pub(crate) email: String,
    /// The login to be used for token authentication
    /// Expected for Cargo
    pub(crate) login: String,
    /// The user's name
    /// Expected for Cargo
    pub(crate) name: String,
    /// The roles for the user
    pub(crate) roles: String,
}

/// The metadata for a crate
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CrateMetadata {
    /// The name of the package
    pub(crate) name: String,
    /// The version of the package being published
    pub(crate) vers: String,
    /// Array of direct dependencies of the package
    pub(crate) deps: Vec<CrateMetadataDependency>,
    /// Set of features defined for the package.
    /// Each feature maps to an array of features or dependencies it enables.
    /// Cargo does not impose limitations on feature names, but crates.io
    /// requires alphanumeric ASCII, `_` or `-` characters.
    pub(crate) features: HashMap<String, Vec<String>>,
    /// List of strings of the authors.
    /// May be empty.
    pub(crate) authors: Vec<String>,
    /// Description field from the manifest.
    /// May be null. crates.io requires at least some content.
    pub(crate) description: Option<String>,
    /// String of the URL to the website for this package's documentation.
    /// May be null.
    pub(crate) documentation: Option<String>,
    /// String of the URL to the website for this package's home page.
    /// May be null.
    pub(crate) homepage: Option<String>,
    /// String of the content of the README file.
    /// May be null.
    pub(crate) readme: Option<String>,
    /// String of a relative path to a README file in the crate.
    /// May be null.
    pub(crate) readme_file: Option<String>,
    /// Array of strings of keywords for the package.
    pub(crate) keywords: Vec<String>,
    /// Array of strings of categories for the package.
    pub(crate) categories: Vec<String>,
    /// String of the license for the package.
    /// May be null. crates.io requires either `license` or `license_file` to be set.
    pub(crate) license: Option<String>,
    /// String of a relative path to a license file in the crate.
    /// May be null.
    pub(crate) license_file: Option<String>,
    /// String of the URL to the website for the source repository of this package.
    /// May be null.
    pub(crate) repository: Option<String>,
    /// Optional object of "status" badges. Each value is an object of
    /// arbitrary string to string mappings.
    /// crates.io has special interpretation of the format of the badges.
    pub(crate) badges: HashMap<String, serde_json::Value>,
    /// The `links` string value from the package's manifest, or null if not
    /// specified. This field is optional and defaults to null.
    pub(crate) links: Option<String>,
    /// The minimal supported Rust version (optional)
    /// This must be a valid version requirement without an operator (e.g. no `=`)
    pub(crate) rust_version: Option<String>,
}

impl CrateMetadata {
    /// Validate the crate's metadata
    pub(crate) fn validate(&self) -> Result<CrateUploadResult, ApiError> {
        self.validate_name()?;
        Ok(CrateUploadResult::default())
    }

    /// Validates the package name
    fn validate_name(&self) -> Result<(), ApiError> {
        if self.name.is_empty() {
            return validation_error("Name must not be empty");
        }
        if self.name.len() > 64 {
            return validation_error("Name must not exceed 64 characters");
        }
        for (i, c) in self.name.chars().enumerate() {
            match (i, c) {
                (0, c) if !c.is_ascii_alphabetic() => {
                    return validation_error("Name must start with an ASCII letter");
                }
                (_, c) if !c.is_ascii_alphanumeric() && c != '-' && c != '_' => {
                    return validation_error("Name must only contain alphanumeric, -, _");
                }
                _ => { /* this is ok */ }
            }
        }
        Ok(())
    }
}

/// Creates a validation error
pub(crate) fn validation_error(details: &str) -> Result<(), ApiError> {
    Err(specialize(error_invalid_request(), details.to_string()))
}

/// The kind of dependency
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DependencyKind {
    /// A normal dependency
    #[default]
    #[serde(rename = "normal")]
    Normal,
    /// A dev dependency (for tests, etc.)
    #[serde(rename = "dev")]
    Dev,
    /// A build dependency (for build.rs)
    #[serde(rename = "build")]
    Build,
}

impl FromStr for DependencyKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(Self::Normal),
            "dev" => Ok(Self::Dev),
            "build" => Ok(Self::Build),
            _ => Err(()),
        }
    }
}

/// A dependency for a crate
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CrateMetadataDependency {
    /// Name of the dependency.
    /// If the dependency is renamed from the original package name,
    /// this is the original name. The new package name is stored in
    /// the `explicit_name_in_toml` field.
    pub(crate) name: String,
    /// The semver requirement for this dependency
    pub(crate) version_req: String,
    /// Array of features (as strings) enabled for this dependency
    pub(crate) features: Vec<String>,
    /// Boolean of whether this is an optional dependency
    pub(crate) optional: bool,
    /// Boolean of whether default features are enabled
    pub(crate) default_features: bool,
    /// The target platform for the dependency.
    /// null if not a target dependency.
    /// Otherwise, a string such as "cfg(windows)".
    pub(crate) target: Option<String>,
    /// The dependency kind.
    /// "dev", "build", or "normal".
    pub(crate) kind: DependencyKind,
    /// The URL of the index of the registry where this dependency is
    /// from as a string. If not specified or null, it is assumed the
    /// dependency is in the current registry.
    pub(crate) registry: Option<String>,
    /// If the dependency is renamed, this is a string of the new
    /// package name. If not specified or null, this dependency is not
    /// renamed.
    pub(crate) explicit_name_in_toml: Option<String>,
}

/// The result for the upload of a crate
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct CrateUploadResult {
    /// The warnings
    pub(crate) warnings: CrateUploadWarnings,
}

/// The warnings for the upload of a crate
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct CrateUploadWarnings {
    /// Array of strings of categories that are invalid and ignored
    pub(crate) invalid_categories: Vec<String>,
    /// Array of strings of badge names that are invalid and ignored
    pub(crate) invalid_badges: Vec<String>,
    /// Array of strings of arbitrary warnings to display to the user
    pub(crate) other: Vec<String>,
}

/// The upload data for publishing a crate
pub(crate) struct CrateUploadData {
    /// The metadata
    pub(crate) metadata: CrateMetadata,
    /// The content of the .crate package
    pub(crate) content: Vec<u8>,
}

impl CrateUploadData {
    /// Deserialize the content of an input payload
    pub(crate) fn new(buffer: &[u8]) -> Result<Self, ApiError> {
        let mut cursor = Cursor::new(buffer);
        // read the metadata
        let metadata_length = cursor.read_u32::<LittleEndian>()? as usize;
        let metadata_buffer = &buffer[4..(4 + metadata_length)];
        let metadata = serde_json::from_slice(metadata_buffer)?;
        // read the content
        cursor.set_position(4 + metadata_length as u64);
        let content_length = cursor.read_u32::<LittleEndian>()? as usize;
        let mut content = vec![0_u8; content_length];
        content.copy_from_slice(&buffer[(4 + metadata_length + 4)..]);
        Ok(Self { metadata, content })
    }

    /// Builds the metadata to be index for this version
    pub(crate) fn build_index_data(&self) -> IndexCrateMetadata {
        let cksum = sha256(&self.content);
        IndexCrateMetadata {
            name: self.metadata.name.clone(),
            vers: self.metadata.vers.clone(),
            deps: self.metadata.deps.iter().map(IndexCrateDependency::from).collect(),
            cksum,
            features: HashMap::new(),
            yanked: false,
            links: self.metadata.links.clone(),
            v: Some(2),
            features2: Some(self.metadata.features.clone()),
            rust_version: self.metadata.rust_version.clone(),
        }
    }
}

/// The metadata for a crate inside the index
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IndexCrateMetadata {
    /// The name of the package
    pub(crate) name: String,
    /// The version of the package this row is describing.
    /// This must be a valid version number according to the Semantic
    /// Versioning 2.0.0 spec at [https://semver.org/](https://semver.org/).
    pub(crate) vers: String,
    /// Array of direct dependencies of the package
    pub(crate) deps: Vec<IndexCrateDependency>,
    /// A SHA256 checksum of the `.crate` file.
    pub(crate) cksum: String,
    /// Set of features defined for the package.
    /// Each feature maps to an array of features or dependencies it enables.
    pub(crate) features: HashMap<String, Vec<String>>,
    /// Boolean of whether this version has been yanked.
    pub(crate) yanked: bool,
    /// The `links` string value from the package's manifest, or null if not
    /// specified. This field is optional and defaults to null.
    pub(crate) links: Option<String>,
    /// An unsigned 32-bit integer value indicating the schema version of this
    /// entry.
    ///
    /// If this not specified, it should be interpreted as the default of 1.
    ///
    /// Cargo (starting with version 1.51) will ignore versions it does not
    /// recognize. This provides a method to safely introduce changes to index
    /// entries and allow older versions of cargo to ignore newer entries it
    /// doesn't understand. Versions older than 1.51 ignore this field, and
    /// thus may misinterpret the meaning of the index entry.
    ///
    /// The current values are:
    ///
    /// * 1: The schema as documented here, not including newer additions.
    ///   This is honored in Rust version 1.51 and newer.
    /// * 2: The addition of the `features2` field.
    ///   This is honored in Rust version 1.60 and newer.
    pub(crate) v: Option<u32>,
    /// This optional field contains features with new, extended syntax.
    /// Specifically, namespaced features (`dep:`) and weak dependencies
    /// (`pkg?/feat`).
    ///
    /// This is separated from `features` because versions older than 1.19
    /// will fail to load due to not being able to parse the new syntax, even
    /// with a `Cargo.lock` file.
    ///
    /// Cargo will merge any values listed here with the "features" field.
    ///
    /// If this field is included, the "v" field should be set to at least 2.
    ///
    /// Registries are not required to use this field for extended feature
    /// syntax, they are allowed to include those in the "features" field.
    /// Using this is only necessary if the registry wants to support cargo
    /// versions older than 1.19, which in practice is only crates.io since
    /// those older versions do not support other registries.
    pub(crate) features2: Option<HashMap<String, Vec<String>>>,
    /// The minimal supported Rust version (optional)
    /// This must be a valid version requirement without an operator (e.g. no `=`)
    pub(crate) rust_version: Option<String>,
}

impl IndexCrateMetadata {
    /// Gets the value associated to a requested feature
    pub(crate) fn get_feature(&self, feature: &str) -> Option<&[String]> {
        self.features2
            .as_ref()
            .and_then(|r| r.get(feature))
            .or_else(|| self.features.get(feature))
            .map(Vec::as_slice)
    }
}

/// A dependency for a crate in the index
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IndexCrateDependency {
    /// Name of the dependency.
    /// If the dependency is renamed from the original package name,
    /// this is the original name. The new package name is stored in
    /// the `package` field.
    pub(crate) name: String,
    /// The semver requirement for this dependency.
    /// This must be a valid version requirement defined at
    /// [https://github.com/steveklabnik/semver#requirements](https://github.com/steveklabnik/semver#requirements).
    pub(crate) req: String,
    /// Array of features (as strings) enabled for this dependency
    pub(crate) features: Vec<String>,
    /// Boolean of whether this is an optional dependency
    pub(crate) optional: bool,
    /// Boolean of whether default features are enabled
    pub(crate) default_features: bool,
    /// The target platform for the dependency.
    /// null if not a target dependency.
    /// Otherwise, a string such as "cfg(windows)".
    pub(crate) target: Option<String>,
    /// The dependency kind.
    /// "dev", "build", or "normal".
    pub(crate) kind: DependencyKind,
    /// The URL of the index of the registry where this dependency is
    /// from as a string. If not specified or null, it is assumed the
    /// dependency is in the current registry.
    pub(crate) registry: Option<String>,
    /// If the dependency is renamed, this is a string of the new
    /// package name. If not specified or null, this dependency is not
    /// renamed.
    pub(crate) package: Option<String>,
}

impl IndexCrateDependency {
    /// Gets the crate name for this dependency
    #[must_use]
    pub(crate) fn get_name(&self) -> &str {
        self.package.as_deref().unwrap_or(&self.name)
    }

    /// Gets whether this dependency is active, for the specified targets and features
    #[must_use]
    pub(crate) fn is_active_for(&self, active_targets: &[String], active_features: &[&str]) -> bool {
        let is_in_targets = self.target.as_ref().is_none_or(|target_spec| {
            target_spec.strip_prefix("cfg(").map_or_else(
                || active_targets.contains(target_spec),
                |rest| {
                    let _cfg_spec = &rest[..rest.len() - 1];
                    // FIXME
                    false
                },
            )
        });
        if !is_in_targets {
            // not for an active target
            return false;
        }
        if !self.optional {
            // not optional
            return true;
        }
        let name = self.get_name();
        active_features.iter().any(|feature| {
            feature.strip_prefix("dep:").is_some_and(|suffix| name == suffix)
                || feature.find('/').is_some_and(|i| &feature[..i] == name)
        })
    }
}

impl From<&CrateMetadataDependency> for IndexCrateDependency {
    fn from(dep: &CrateMetadataDependency) -> Self {
        Self {
            name: dep.explicit_name_in_toml.as_ref().unwrap_or(&dep.name).clone(),
            req: dep.version_req.clone(),
            features: dep.features.clone(),
            optional: dep.optional,
            default_features: dep.default_features,
            target: dep.target.clone(),
            kind: dep.kind,
            registry: dep.registry.clone(),
            package: if dep.explicit_name_in_toml.is_some() {
                Some(dep.name.clone())
            } else {
                None
            },
        }
    }
}
