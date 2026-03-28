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
pub mod dependency;
pub mod endpoint;
pub mod error;
pub mod manifest;
pub mod result;
pub mod scope;
pub mod validation;
pub mod version;

// Re-exports for convenience
pub use component::{Component, ComponentKind};
pub use endpoint::{AdapterCapabilities, Endpoint, KnownEndpoint};
pub use error::{Error, Result};
pub use result::{
    ComponentInstall, ComponentStatus, ExportFormat, ExportPlan, ImportArtifact, ImportPlan,
    InstallFile, InstallPlan, InstallResult, StatusResult, UninstallResult, ValidationMessage,
    ValidationResult,
};
pub use scope::Scope;
pub use version::VersionPolicy;

pub use dependency::{ComponentDependency, DependencyResolver};
pub use manifest::{
    Author, BundleManifest, CatalogEntry, CatalogManifest, CompatConfig, ComponentRef, Provenance,
    Tag,
};
pub use validation::{ValidationReport, ValidationSeverity};
