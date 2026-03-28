// Copyright 2025 skillctrl contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Core types and traits for skillctrl.
//!
//! This crate provides the foundational abstractions used across all
//! skillctrl components, including component types, endpoints, scopes,
//! and common error types.

pub mod component;
pub mod endpoint;
pub mod scope;
pub mod version;
pub mod error;
pub mod result;
pub mod dependency;
pub mod validation;
pub mod manifest;

// Re-exports for convenience
pub use component::{Component, ComponentKind};
pub use endpoint::{Endpoint, KnownEndpoint, AdapterCapabilities};
pub use scope::Scope;
pub use version::VersionPolicy;
pub use error::{Error, Result};
pub use result::{
    InstallResult, UninstallResult, StatusResult, ValidationResult,
    InstallPlan, InstallFile, ComponentInstall, ImportPlan, ExportPlan,
    ValidationMessage, ComponentStatus,
    ImportArtifact, ExportFormat,
};

pub use dependency::{ComponentDependency, DependencyResolver};
pub use validation::{ValidationReport, ValidationSeverity};
pub use manifest::{
    BundleManifest, CatalogManifest, CatalogEntry,
    Author, Tag, ComponentRef, CompatConfig, Provenance
};
