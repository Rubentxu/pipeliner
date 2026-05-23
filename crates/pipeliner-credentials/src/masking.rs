//! Secret masking utilities for Pipeliner.

use std::collections::HashSet;
use std::sync::Arc;
use regex::Regex;
use parking_lot::RwLock;

/// A utility for masking sensitive values in output.
#[derive(Debug, Clone)]
pub struct SecretMasker {
    /// Regex patterns that match sensitive data
    patterns: Vec<Regex>,
    /// Variables that should be masked
    masked_vars: Arc<RwLock<HashSet<String>>>,
    /// Whether masking is enabled
    enabled: Arc<RwLock<bool>>,
}

impl Default for SecretMasker {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretMasker {
    /// Creates a new SecretMasker with default patterns.
    #[must_use]
    pub fn new() -> Self {
        let patterns = vec![
            // AWS access keys
            Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
            // Generic API keys
            Regex::new(r"(?i)(api[_-]?key|apikey|secret[_-]?key|access[_-]?token)\s*[:=]\s*[a-zA-Z0-9_\-]{20,}").unwrap(),
            // Generic secrets
            Regex::new(r"(?i)(password|passwd|pwd|secret)\s*[:=]\s*[^\s]{8,}").unwrap(),
            // Bearer tokens
            Regex::new(r"Bearer\s+[a-zA-Z0-9_\-\.]+").unwrap(),
            // JWT tokens
            Regex::new(r"eyJ[a-zA-Z0-9_\-]+\.eyJ[a-zA-Z0-9_\-]+\.[a-zA-Z0-9_\-]+").unwrap(),
            // GitHub tokens
            Regex::new(r"gh[pousr]_[a-zA-Z0-9_]{36,}").unwrap(),
            // Private keys
            Regex::new(r"-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----").unwrap(),
        ];

        Self {
            patterns,
            masked_vars: Arc::new(RwLock::new(HashSet::new())),
            enabled: Arc::new(RwLock::new(true)),
        }
    }

    /// Creates a new SecretMasker with no patterns (empty masker).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            patterns: Vec::new(),
            masked_vars: Arc::new(RwLock::new(HashSet::new())),
            enabled: Arc::new(RwLock::new(true)),
        }
    }

    /// Masks sensitive patterns in the input string.
    ///
    /// Replaces matches with `***`.
    #[must_use]
    pub fn mask(&self, input: &str) -> String {
        if !*self.enabled.read() {
            return input.to_string();
        }

        let mut result = input.to_string();
        for pattern in &self.patterns {
            result = pattern.replace_all(&result, "***").to_string();
        }

        // Also mask registered variables using simple string replacement
        let masked_vars = self.masked_vars.read();
        for var in masked_vars.iter() {
            // Replace $VAR patterns (literal string replacement)
            let bare_pattern = format!("${}", var);
            result = result.replace(&bare_pattern, "***");

            // Replace ${VAR} patterns
            let braced_pattern = format!("${{{}}}", var);
            result = result.replace(&braced_pattern, "***");
        }

        result
    }

    /// Registers a variable name to be masked.
    pub fn register_variable(&self, var: impl Into<String>) {
        self.masked_vars.write().insert(var.into());
    }

    /// Unregisters a variable name from masking.
    pub fn unregister_variable(&self, var: &str) {
        self.masked_vars.write().remove(var);
    }

    /// Checks if a variable is registered for masking.
    #[must_use]
    pub fn is_registered(&self, var: &str) -> bool {
        self.masked_vars.read().contains(var)
    }

    /// Enables or disables masking.
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }

    /// Returns whether masking is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    /// Adds a custom pattern to mask.
    ///
    /// # Panics
    ///
    /// Panics if the pattern is not a valid regex.
    pub fn add_pattern(&mut self, pattern: &str) {
        let regex = Regex::new(pattern).expect("Invalid regex pattern");
        // Skip if it's already in the list
        if !self.patterns.iter().any(|p: &Regex| p.as_str() == regex.as_str()) {
            self.patterns.push(regex);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_aws_key() {
        let masker = SecretMasker::new();
        let input = "AWS_ACCESS_KEY=AKIAIOSFODNN7EXAMPLE";
        let masked = masker.mask(input);
        assert_eq!(masked, "AWS_ACCESS_KEY=***");
    }

    #[test]
    fn test_mask_api_key() {
        let masker = SecretMasker::new();
        let input = "api_key=sk-1234567890abcdefghijklmnopqrstuvwxyz";
        let masked = masker.mask(input);
        assert!(masked.contains("***"));
    }

    #[test]
    fn test_mask_bearer_token() {
        let masker = SecretMasker::new();
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let masked = masker.mask(input);
        assert!(masked.contains("***"));
    }

    #[test]
    fn test_mask_jwt_token() {
        let masker = SecretMasker::new();
        let input = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let masked = masker.mask(input);
        assert!(masked.contains("***"));
    }

    #[test]
    fn test_mask_registered_variable() {
        let masker = SecretMasker::new();
        masker.register_variable("MY_SECRET");
        let input = "The secret is $MY_SECRET";
        let masked = masker.mask(input);
        assert_eq!(masked, "The secret is ***");
    }

    #[test]
    fn test_mask_registered_variable_braces() {
        let masker = SecretMasker::new();
        masker.register_variable("MY_SECRET");
        let input = "The secret is ${MY_SECRET}";
        let masked = masker.mask(input);
        assert_eq!(masked, "The secret is ***");
    }

    #[test]
    fn test_mask_disabled() {
        let masker = SecretMasker::new();
        masker.set_enabled(false);
        let input = "AWS_ACCESS_KEY=AKIAIOSFODNN7EXAMPLE";
        let masked = masker.mask(input);
        assert_eq!(masked, input);
    }

    #[test]
    fn test_mask_empty_no_changes() {
        let masker = SecretMasker::empty();
        let input = "AKIAIOSFODNN7EXAMPLE";
        let masked = masker.mask(input);
        assert_eq!(masked, input);
    }

    #[test]
    fn test_is_registered() {
        let masker = SecretMasker::new();
        assert!(!masker.is_registered("VAR"));
        masker.register_variable("VAR");
        assert!(masker.is_registered("VAR"));
    }

    #[test]
    fn test_unregister_variable() {
        let masker = SecretMasker::new();
        masker.register_variable("VAR");
        assert!(masker.is_registered("VAR"));
        masker.unregister_variable("VAR");
        assert!(!masker.is_registered("VAR"));
    }

    #[test]
    fn test_add_custom_pattern() {
        let mut masker = SecretMasker::empty();
        masker.add_pattern(r"mykey:\s*\w+");
        let input = "mykey: abc123def456";
        let masked = masker.mask(input);
        // The pattern matches "mykey: " followed by word characters
        // But since the pattern doesn't match the whole string, only the matched portion is replaced
        assert!(masked.contains("***"));
    }
}
