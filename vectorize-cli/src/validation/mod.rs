//! Configuration validation module
//!
//! Provides comprehensive validation for Vector configurations:
//! - Layer 1: TOML syntax validation
//! - Layer 2: Schema validation (known fields, types)
//! - Layer 3: Vector binary validation (via `vector validate`)
//! - Layer 4: Functional testing with sample data
//!
//! Also includes component validation (sources, transforms, sinks exist)

pub mod functional_test;

pub use functional_test::{
    FunctionalTestService, FunctionalTestRequest, FunctionalTestResult,
    TestStatus, TransformResult, FunctionalTestError,
};

use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the config is valid
    pub valid: bool,
    /// List of errors found
    pub errors: Vec<ValidationError>,
    /// List of warnings
    pub warnings: Vec<ValidationWarning>,
    /// Summary message
    pub message: String,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: vec![],
            warnings: vec![],
            message: "Configuration is valid".to_string(),
        }
    }
    
    pub fn failure(message: String) -> Self {
        Self {
            valid: false,
            errors: vec![],
            warnings: vec![],
            message,
        }
    }
    
    pub fn with_error(mut self, error: ValidationError) -> Self {
        self.valid = false;
        self.message = error.message.clone();
        self.errors.push(error);
        self
    }
    
    pub fn with_warning(mut self, warning: ValidationWarning) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Line number (if applicable)
    pub line: Option<usize>,
    /// Column number (if applicable)
    pub column: Option<usize>,
    /// Component ID (if applicable)
    pub component: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Component ID (if applicable)
    pub component: Option<String>,
}

/// Configuration validator
pub struct ConfigValidator {
    /// Path to Vector binary
    vector_bin: Option<String>,
}

impl ConfigValidator {
    /// Create a new validator
    pub fn new(vector_bin: Option<String>) -> Self {
        Self { vector_bin }
    }
    
    /// Validate a configuration string
    pub fn validate(&self, config: &str) -> ValidationResult {
        let mut result = ValidationResult::success();
        
        // Step 1: TOML syntax validation
        match self.validate_toml_syntax(config) {
            Ok(_) => {}
            Err(e) => {
                return result.with_error(e);
            }
        }
        
        // Step 2: Schema validation (basic checks)
        let schema_warnings = self.validate_schema(config);
        for warning in schema_warnings {
            result = result.with_warning(warning);
        }
        
        // Step 3: Component validation
        match self.validate_components(config) {
            Ok(warnings) => {
                for warning in warnings {
                    result = result.with_warning(warning);
                }
            }
            Err(e) => {
                return result.with_error(e);
            }
        }
        
        result
    }
    
    /// Validate TOML syntax
    fn validate_toml_syntax(&self, config: &str) -> Result<toml::Value, ValidationError> {
        match toml::from_str::<toml::Value>(config) {
            Ok(value) => Ok(value),
            Err(e) => {
                // Parse the TOML error to extract line/column
                let message = e.message();
                let span = e.span();
                
                let (line, column) = if let Some(span) = span {
                    // Convert byte offset to line/column
                    let line = config[..span.start].matches('\n').count() + 1;
                    let last_newline = config[..span.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let column = span.start - last_newline + 1;
                    (Some(line), Some(column))
                } else {
                    (None, None)
                };
                
                Err(ValidationError {
                    code: "TOML_SYNTAX".to_string(),
                    message: message.to_string(),
                    line,
                    column,
                    component: None,
                })
            }
        }
    }
    
    /// Validate schema (basic structure checks)
    fn validate_schema(&self, config: &str) -> Vec<ValidationWarning> {
        let mut warnings = Vec::new();
        
        let value: toml::Value = match toml::from_str(config) {
            Ok(v) => v,
            Err(_) => return warnings, // Already caught in syntax check
        };
        
        let table = match value.as_table() {
            Some(t) => t,
            None => return warnings,
        };
        
        // Check for unknown top-level keys
        let known_keys = ["api", "sources", "transforms", "sinks", "tests", "enrichment_tables", "secret"];
        for key in table.keys() {
            if !known_keys.contains(&key.as_str()) {
                warnings.push(ValidationWarning {
                    code: "UNKNOWN_KEY".to_string(),
                    message: format!("Unknown top-level key: '{}'", key),
                    component: None,
                });
            }
        }
        
        // Check if there are any sources
        if !table.contains_key("sources") {
            warnings.push(ValidationWarning {
                code: "NO_SOURCES".to_string(),
                message: "Configuration has no sources defined".to_string(),
                component: None,
            });
        }
        
        // Check if there are any sinks
        if !table.contains_key("sinks") {
            warnings.push(ValidationWarning {
                code: "NO_SINKS".to_string(),
                message: "Configuration has no sinks defined".to_string(),
                component: None,
            });
        }
        
        warnings
    }
    
    /// Validate component references
    fn validate_components(&self, config: &str) -> Result<Vec<ValidationWarning>, ValidationError> {
        let mut warnings = Vec::new();
        
        let value: toml::Value = match toml::from_str(config) {
            Ok(v) => v,
            Err(_) => return Ok(warnings),
        };
        
        let table = match value.as_table() {
            Some(t) => t,
            None => return Ok(warnings),
        };
        
        // Collect all component IDs
        let mut source_ids: Vec<String> = Vec::new();
        let mut transform_ids: Vec<String> = Vec::new();
        
        if let Some(sources) = table.get("sources").and_then(|v| v.as_table()) {
            source_ids.extend(sources.keys().cloned());
        }
        
        if let Some(transforms) = table.get("transforms").and_then(|v| v.as_table()) {
            transform_ids.extend(transforms.keys().cloned());
        }
        
        let all_output_ids: Vec<&str> = source_ids.iter()
            .chain(transform_ids.iter())
            .map(|s| s.as_str())
            .collect();
        
        // Check transform inputs
        if let Some(transforms) = table.get("transforms").and_then(|v| v.as_table()) {
            for (name, transform) in transforms {
                if let Some(inputs) = transform.get("inputs").and_then(|v| v.as_array()) {
                    for input in inputs {
                        if let Some(input_str) = input.as_str() {
                            if !all_output_ids.contains(&input_str) {
                                return Err(ValidationError {
                                    code: "INVALID_INPUT".to_string(),
                                    message: format!(
                                        "Transform '{}' references unknown input '{}'",
                                        name, input_str
                                    ),
                                    line: None,
                                    column: None,
                                    component: Some(name.clone()),
                                });
                            }
                        }
                    }
                } else {
                    warnings.push(ValidationWarning {
                        code: "MISSING_INPUTS".to_string(),
                        message: format!("Transform '{}' has no inputs defined", name),
                        component: Some(name.clone()),
                    });
                }
                
                // Check type is specified
                if transform.get("type").is_none() {
                    return Err(ValidationError {
                        code: "MISSING_TYPE".to_string(),
                        message: format!("Transform '{}' has no type specified", name),
                        line: None,
                        column: None,
                        component: Some(name.clone()),
                    });
                }
            }
        }
        
        // Check sink inputs
        if let Some(sinks) = table.get("sinks").and_then(|v| v.as_table()) {
            for (name, sink) in sinks {
                if let Some(inputs) = sink.get("inputs").and_then(|v| v.as_array()) {
                    for input in inputs {
                        if let Some(input_str) = input.as_str() {
                            if !all_output_ids.contains(&input_str) {
                                return Err(ValidationError {
                                    code: "INVALID_INPUT".to_string(),
                                    message: format!(
                                        "Sink '{}' references unknown input '{}'",
                                        name, input_str
                                    ),
                                    line: None,
                                    column: None,
                                    component: Some(name.clone()),
                                });
                            }
                        }
                    }
                } else {
                    warnings.push(ValidationWarning {
                        code: "MISSING_INPUTS".to_string(),
                        message: format!("Sink '{}' has no inputs defined", name),
                        component: Some(name.clone()),
                    });
                }
                
                // Check type is specified
                if sink.get("type").is_none() {
                    return Err(ValidationError {
                        code: "MISSING_TYPE".to_string(),
                        message: format!("Sink '{}' has no type specified", name),
                        line: None,
                        column: None,
                        component: Some(name.clone()),
                    });
                }
            }
        }
        
        // Check sources have type
        if let Some(sources) = table.get("sources").and_then(|v| v.as_table()) {
            for (name, source) in sources {
                if source.get("type").is_none() {
                    return Err(ValidationError {
                        code: "MISSING_TYPE".to_string(),
                        message: format!("Source '{}' has no type specified", name),
                        line: None,
                        column: None,
                        component: Some(name.clone()),
                    });
                }
            }
        }
        
        Ok(warnings)
    }
    
    /// Run Vector's validate command
    pub fn validate_with_vector(&self, config: &str) -> ValidationResult {
        // First do our own validation
        let result = self.validate(config);
        if !result.valid {
            return result;
        }
        
        // Find Vector binary
        let vector_bin = self.vector_bin.clone().unwrap_or_else(|| "vector".to_string());
        
        // Write config to temp file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("vectorize-validate-{}.toml", std::process::id()));
        
        if let Err(e) = std::fs::write(&temp_file, config) {
            return ValidationResult::failure(format!("Failed to write temp file: {}", e));
        }
        
        // Run vector validate
        let output = Command::new(&vector_bin)
            .args(["validate", "--config-toml", temp_file.to_str().unwrap_or("")])
            .output();
        
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);
        
        match output {
            Ok(output) => {
                if output.status.success() {
                    info!("Vector validation passed");
                    result
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    
                    // Parse Vector's error output
                    let error_msg = if !stderr.is_empty() {
                        stderr.to_string()
                    } else {
                        stdout.to_string()
                    };
                    
                    warn!("Vector validation failed: {}", error_msg);
                    
                    result.with_error(ValidationError {
                        code: "VECTOR_VALIDATE".to_string(),
                        message: format!("Vector validation failed: {}", error_msg.trim()),
                        line: None,
                        column: None,
                        component: None,
                    })
                }
            }
            Err(e) => {
                warn!("Failed to run vector validate: {}", e);
                // Vector binary not available - return our validation result with a warning
                result.with_warning(ValidationWarning {
                    code: "VECTOR_NOT_FOUND".to_string(),
                    message: format!("Could not run Vector validation: {}", e),
                    component: None,
                })
            }
        }
    }
    
    /// Validate a config file
    #[allow(dead_code)]
    pub fn validate_file(&self, path: &Path) -> ValidationResult {
        match std::fs::read_to_string(path) {
            Ok(config) => self.validate_with_vector(&config),
            Err(e) => ValidationResult::failure(format!("Failed to read config file: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn validator() -> ConfigValidator {
        ConfigValidator::new(None)
    }
    
    // =========================================================================
    // Valid Configuration Tests
    // =========================================================================
    
    #[test]
    fn test_valid_config() {
        let config = r#"
[sources.demo]
type = "demo_logs"
format = "json"

[sinks.console]
type = "console"
inputs = ["demo"]
"#;
        
        let result = validator().validate(config);
        
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }
    
    #[test]
    fn test_valid_with_transform() {
        let config = r#"
[sources.logs]
type = "demo_logs"

[transforms.filter]
type = "filter"
inputs = ["logs"]
condition = "true"

[sinks.console]
type = "console"
inputs = ["filter"]
"#;
        
        let result = validator().validate(config);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }
    
    #[test]
    fn test_valid_multiple_sources() {
        let config = r#"
[sources.source1]
type = "demo_logs"

[sources.source2]
type = "demo_logs"

[sinks.console]
type = "console"
inputs = ["source1", "source2"]
"#;
        
        let result = validator().validate(config);
        assert!(result.valid);
    }
    
    #[test]
    fn test_valid_chain_transforms() {
        let config = r#"
[sources.logs]
type = "demo_logs"

[transforms.transform1]
type = "filter"
inputs = ["logs"]

[transforms.transform2]
type = "remap"
inputs = ["transform1"]

[sinks.output]
type = "console"
inputs = ["transform2"]
"#;
        
        let result = validator().validate(config);
        assert!(result.valid);
    }
    
    // =========================================================================
    // TOML Syntax Error Tests
    // =========================================================================
    
    #[test]
    fn test_invalid_toml() {
        let config = r#"
[sources.demo
type = "demo_logs"
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].code, "TOML_SYNTAX");
    }
    
    #[test]
    fn test_invalid_toml_missing_quotes() {
        let config = r#"
[sources.demo]
type = demo_logs
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "TOML_SYNTAX");
    }
    
    #[test]
    fn test_invalid_toml_unclosed_string() {
        let config = r#"
[sources.demo]
type = "demo_logs
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "TOML_SYNTAX");
    }
    
    #[test]
    fn test_invalid_toml_duplicate_key() {
        let config = r#"
[sources.demo]
type = "demo_logs"
type = "file"
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "TOML_SYNTAX");
    }
    
    #[test]
    fn test_toml_error_line_number() {
        let config = r#"
[sources.demo]
type = "demo_logs"

[sources.bad
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert!(result.errors[0].line.is_some());
        // Error should be on line 5 (0-indexed would be 4)
        let line = result.errors[0].line.unwrap();
        assert!(line >= 4 && line <= 6);
    }
    
    // =========================================================================
    // Missing Type Tests
    // =========================================================================
    
    #[test]
    fn test_missing_source_type() {
        let config = r#"
[sources.demo]
format = "json"

[sinks.console]
type = "console"
inputs = ["demo"]
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "MISSING_TYPE");
        assert!(result.errors[0].component.as_ref().unwrap().contains("demo"));
    }
    
    #[test]
    fn test_missing_transform_type() {
        let config = r#"
[sources.logs]
type = "demo_logs"

[transforms.filter]
inputs = ["logs"]

[sinks.console]
type = "console"
inputs = ["filter"]
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "MISSING_TYPE");
        assert!(result.errors[0].component.as_ref().unwrap().contains("filter"));
    }
    
    #[test]
    fn test_missing_sink_type() {
        let config = r#"
[sources.logs]
type = "demo_logs"

[sinks.output]
inputs = ["logs"]
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "MISSING_TYPE");
        assert!(result.errors[0].component.as_ref().unwrap().contains("output"));
    }
    
    // =========================================================================
    // Invalid Input Reference Tests
    // =========================================================================
    
    #[test]
    fn test_missing_input() {
        let config = r#"
[sources.demo]
type = "demo_logs"

[sinks.console]
type = "console"
inputs = ["nonexistent"]
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "INVALID_INPUT");
    }
    
    #[test]
    fn test_transform_missing_input() {
        let config = r#"
[sources.logs]
type = "demo_logs"

[transforms.filter]
type = "filter"
inputs = ["missing_source"]

[sinks.console]
type = "console"
inputs = ["filter"]
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "INVALID_INPUT");
    }
    
    #[test]
    fn test_sink_reference_missing_transform() {
        let config = r#"
[sources.logs]
type = "demo_logs"

[sinks.console]
type = "console"
inputs = ["missing_transform"]
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "INVALID_INPUT");
    }
    
    #[test]
    fn test_partial_valid_inputs() {
        let config = r#"
[sources.logs]
type = "demo_logs"

[sinks.console]
type = "console"
inputs = ["logs", "nonexistent"]
"#;
        
        let result = validator().validate(config);
        
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, "INVALID_INPUT");
    }
    
    // =========================================================================
    // Warning Tests
    // =========================================================================
    
    #[test]
    fn test_unknown_top_level_key_warning() {
        let config = r#"
[api]
enabled = true

[unknown_section]
foo = "bar"

[sources.demo]
type = "demo_logs"

[sinks.console]
type = "console"
inputs = ["demo"]
"#;
        
        let result = validator().validate(config);
        
        assert!(result.valid);
        let has_unknown_warning = result.warnings.iter()
            .any(|w| w.code == "UNKNOWN_KEY");
        assert!(has_unknown_warning);
    }
    
    #[test]
    fn test_no_sources_warning() {
        let config = r#"
[api]
enabled = true
"#;
        
        let result = validator().validate(config);
        
        assert!(result.valid);  // Valid TOML but warns
        let has_warning = result.warnings.iter()
            .any(|w| w.code == "NO_SOURCES");
        assert!(has_warning);
    }
    
    #[test]
    fn test_no_sinks_warning() {
        let config = r#"
[sources.demo]
type = "demo_logs"
"#;
        
        let result = validator().validate(config);
        
        assert!(result.valid);  // Valid TOML but warns
        let has_warning = result.warnings.iter()
            .any(|w| w.code == "NO_SINKS");
        assert!(has_warning);
    }
    
    #[test]
    fn test_missing_inputs_warning() {
        let config = r#"
[sources.logs]
type = "demo_logs"

[sinks.console]
type = "console"
"#;  // No inputs field on sink
        
        let result = validator().validate(config);
        
        assert!(result.valid);
        let has_warning = result.warnings.iter()
            .any(|w| w.code == "MISSING_INPUTS");
        assert!(has_warning);
    }
    
    #[test]
    fn test_warnings() {
        let config = r#"
[api]
enabled = true

[unknown_section]
foo = "bar"
"#;
        
        let result = validator().validate(config);
        
        // Valid syntax but with warnings
        assert!(result.valid);
        assert!(!result.warnings.is_empty());
    }
    
    // =========================================================================
    // Edge Cases
    // =========================================================================
    
    #[test]
    fn test_empty_config() {
        let config = "";
        
        let result = validator().validate(config);
        
        // Empty config is valid TOML but has warnings
        assert!(result.valid);
        assert!(!result.warnings.is_empty());  // Should warn about no sources/sinks
    }
    
    #[test]
    fn test_whitespace_only_config() {
        let config = "   \n\n   \t\t\n";
        
        let result = validator().validate(config);
        
        assert!(result.valid);
    }
    
    #[test]
    fn test_comment_only_config() {
        let config = r#"
# This is a comment
# Another comment
"#;
        
        let result = validator().validate(config);
        
        assert!(result.valid);
    }
    
    #[test]
    fn test_valid_complex_config() {
        let config = r#"
[api]
enabled = true
address = "0.0.0.0:8686"

[sources.demo_logs]
type = "demo_logs"
format = "json"
interval = 1

[sources.file_logs]
type = "file"

[transforms.parse_json]
type = "remap"
inputs = ["demo_logs", "file_logs"]
source = ". = parse_json!(.message)"

[transforms.filter_errors]
type = "filter"
inputs = ["parse_json"]
condition = '.level == "error"'

[sinks.console]
type = "console"
inputs = ["filter_errors"]

[sinks.blackhole]
type = "blackhole"
inputs = ["parse_json"]
"#;
        
        let result = validator().validate(config);
        assert!(result.valid);
    }
    
    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success();
        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }
    
    #[test]
    fn test_validation_result_failure() {
        let result = ValidationResult::failure("Test failure".to_string());
        assert!(!result.valid);
        assert_eq!(result.message, "Test failure");
    }
    
    #[test]
    fn test_validation_result_with_error() {
        let result = ValidationResult::success()
            .with_error(ValidationError {
                code: "TEST".to_string(),
                message: "Test error".to_string(),
                line: Some(10),
                column: Some(5),
                component: Some("test_component".to_string()),
            });
        
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "TEST");
        assert_eq!(result.errors[0].line, Some(10));
    }
    
    #[test]
    fn test_validation_result_with_warning() {
        let result = ValidationResult::success()
            .with_warning(ValidationWarning {
                code: "WARN".to_string(),
                message: "Test warning".to_string(),
                component: None,
            });
        
        assert!(result.valid);  // Warnings don't invalidate
        assert_eq!(result.warnings.len(), 1);
    }
}
