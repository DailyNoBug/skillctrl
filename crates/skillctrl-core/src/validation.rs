//! Validation types and utilities.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Severity of a validation message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationSeverity {
    /// Information only.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

/// Validation report.
///
/// Contains the results of validating a bundle, component, or other artifact.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// Validation messages.
    pub messages: Vec<ValidationMessage>,
}

impl ValidationReport {
    /// Creates a new empty validation report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a message to the report.
    pub fn add_message(&mut self, message: ValidationMessage) {
        self.messages.push(message);
    }

    /// Adds an info message.
    pub fn info(&mut self, message: impl Into<String>) {
        self.add_message(ValidationMessage {
            severity: ValidationSeverity::Info,
            message: message.into(),
            path: None,
        });
    }

    /// Adds a warning message.
    pub fn warning(&mut self, message: impl Into<String>) {
        self.add_message(ValidationMessage {
            severity: ValidationSeverity::Warning,
            message: message.into(),
            path: None,
        });
    }

    /// Adds an error message.
    pub fn error(&mut self, message: impl Into<String>) {
        self.add_message(ValidationMessage {
            severity: ValidationSeverity::Error,
            message: message.into(),
            path: None,
        });
    }

    /// Adds a message with a path.
    pub fn add_with_path(
        &mut self,
        severity: ValidationSeverity,
        message: impl Into<String>,
        path: PathBuf,
    ) {
        self.add_message(ValidationMessage {
            severity,
            message: message.into(),
            path: Some(path),
        });
    }

    /// Returns true if validation passed (no errors).
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Returns true if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.messages
            .iter()
            .any(|m| m.severity == ValidationSeverity::Error)
    }

    /// Returns true if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        self.messages
            .iter()
            .any(|m| m.severity == ValidationSeverity::Warning)
    }

    /// Returns all error messages.
    pub fn errors(&self) -> Vec<&ValidationMessage> {
        self.messages
            .iter()
            .filter(|m| m.severity == ValidationSeverity::Error)
            .collect()
    }

    /// Returns all warning messages.
    pub fn warnings(&self) -> Vec<&ValidationMessage> {
        self.messages
            .iter()
            .filter(|m| m.severity == ValidationSeverity::Warning)
            .collect()
    }

    /// Merges another validation report into this one.
    pub fn merge(&mut self, other: ValidationReport) {
        self.messages.extend(other.messages);
    }
}

impl IntoIterator for ValidationReport {
    type Item = ValidationMessage;
    type IntoIter = std::vec::IntoIter<ValidationMessage>;

    fn into_iter(self) -> Self::IntoIter {
        self.messages.into_iter()
    }
}

/// A validation message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessage {
    /// Severity level.
    pub severity: ValidationSeverity,
    /// Message.
    pub message: String,
    /// Related path (if any).
    pub path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_report() {
        let mut report = ValidationReport::new();

        report.info("This is info");
        report.warning("This is a warning");
        report.error("This is an error");

        assert!(report.has_errors());
        assert!(report.has_warnings());
        assert_eq!(report.errors().len(), 1);
        assert_eq!(report.warnings().len(), 1);
    }

    #[test]
    fn test_validation_report_valid() {
        let mut report = ValidationReport::new();
        report.info("Just info");

        assert!(report.is_valid());
        assert!(!report.has_errors());
    }
}
