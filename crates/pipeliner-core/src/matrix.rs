//! Matrix execution configuration for parallel pipeline stages.
//!
//! This module provides types for defining matrix-based parallel
//! execution, allowing stages to run with different configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Matrix configuration for parallel execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MatrixConfig {
    /// Matrix axes defining dimensions of parallel execution
    pub axes: Vec<MatrixAxis>,
    /// Exclusions (combinations to skip)
    #[serde(default)]
    pub exclude: Vec<MatrixExclude>,
    /// Agent configuration for matrix cells
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Stage name template for matrix cells
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_template: Option<String>,
}

/// A single axis of the matrix
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MatrixAxis {
    /// Simple string values
    Values {
        /// Axis name
        name: String,
        /// Possible values
        values: Vec<String>,
    },
    /// Numeric range
    Range {
        /// Axis name
        name: String,
        /// Start value (inclusive)
        start: i64,
        /// End value (inclusive)
        end: i64,
        /// Step value
        step: i64,
    },
    /// From a file
    File {
        /// Axis name
        name: String,
        /// File path
        file: String,
        /// Separator
        #[serde(default = "default_separator")]
        separator: String,
    },
    /// From an expression
    Expression {
        /// Axis name
        name: String,
        /// Expression to evaluate
        expression: String,
    },
}

fn default_separator() -> String {
    ",".to_string()
}

/// Exclusion configuration for matrix cells
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixExclude {
    /// Values to exclude for each axis
    pub axes: HashMap<String, String>,
    /// Reason for exclusion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Generated matrix cell
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MatrixCell {
    /// Cell index
    pub index: usize,
    /// Values for each axis
    pub values: HashMap<String, String>,
    /// Cell-specific name
    pub name: String,
}

/// Matrix iterator for generating cells
#[derive(Debug, Clone)]
pub struct MatrixIterator {
    /// Matrix configuration
    config: MatrixConfig,
    /// Generated combinations
    combinations: Vec<HashMap<String, String>>,
    /// Current index
    index: usize,
}

impl MatrixConfig {
    /// Generates all matrix cell combinations
    #[must_use]
    pub fn generate_cells(&self) -> Vec<MatrixCell> {
        let combinations = self.generate_combinations();
        let mut cells = Vec::with_capacity(combinations.len());

        for (index, combo) in combinations.into_iter().enumerate() {
            let name = self.generate_cell_name(&combo, index);
            cells.push(MatrixCell {
                index,
                values: combo,
                name,
            });
        }

        cells
    }

    /// Returns the number of cells that will be generated
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.generate_combinations().len()
    }

    /// Checks if the matrix configuration is valid
    pub fn validate(&self) -> Result<(), crate::ValidationError> {
        if self.axes.is_empty() {
            return Err(crate::ValidationError::InvalidMatrix {
                reason: "matrix must have at least one axis".to_string(),
            });
        }

        for axis in &self.axes {
            match axis {
                MatrixAxis::Values { name, values } => {
                    if values.is_empty() {
                        return Err(crate::ValidationError::InvalidMatrix {
                            reason: format!("axis '{}' must have at least one value", name),
                        });
                    }
                }
                MatrixAxis::Range {
                    name,
                    start,
                    end,
                    step,
                } => {
                    if *step <= 0 {
                        return Err(crate::ValidationError::InvalidMatrix {
                            reason: format!("axis '{}' step must be positive", name),
                        });
                    }
                    if start > end {
                        return Err(crate::ValidationError::InvalidMatrix {
                            reason: format!("axis '{}' start must be <= end", name),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Generates all combinations of axis values
    fn generate_combinations(&self) -> Vec<HashMap<String, String>> {
        if self.axes.is_empty() {
            return Vec::new();
        }

        let axis_values: Vec<(String, Vec<String>)> = self
            .axes
            .iter()
            .filter_map(|axis| match axis {
                MatrixAxis::Values { name, values } => Some((name.clone(), values.clone())),
                MatrixAxis::Range {
                    name,
                    start,
                    end,
                    step,
                } => {
                    let values: Vec<String> = (*start..=*end)
                        .step_by(*step as usize)
                        .map(|v| v.to_string())
                        .collect();
                    Some((name.clone(), values))
                }
                _ => None,
            })
            .collect();

        if axis_values.is_empty() {
            return Vec::new();
        }

        let mut combinations = Vec::new();
        self.cartesian_product(&axis_values, 0, &mut HashMap::new(), &mut combinations);
        combinations
    }

    /// Recursive helper for cartesian product
    fn cartesian_product(
        &self,
        axes: &[(String, Vec<String>)],
        index: usize,
        current: &mut HashMap<String, String>,
        results: &mut Vec<HashMap<String, String>>,
    ) {
        if index == axes.len() {
            let combo = current.clone();
            if let Some(excluded) = self.find_exclusion(&combo) {
                if excluded {
                    return;
                }
            }
            results.push(combo);
            return;
        }

        let (name, values) = &axes[index];
        for value in values {
            current.insert(name.clone(), value.clone());
            self.cartesian_product(axes, index + 1, current, results);
            current.remove(name);
        }
    }

    /// Checks if a combination should be excluded
    fn find_exclusion(&self, combo: &HashMap<String, String>) -> Option<bool> {
        for exclusion in &self.exclude {
            let mut matches = true;
            for (key, value) in &exclusion.axes {
                if combo.get(key) != Some(value) {
                    matches = false;
                    break;
                }
            }
            if matches {
                return Some(true);
            }
        }
        None
    }

    /// Generates a cell name from combination
    fn generate_cell_name(&self, combo: &HashMap<String, String>, index: usize) -> String {
        if let Some(template) = &self.name_template {
            template
                .replace("{idx}", &index.to_string())
                .replace("{", "")
                .replace("}", "")
        } else {
            let parts: Vec<String> = combo.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            if parts.is_empty() {
                format!("cell-{}", index)
            } else {
                parts.join("-")
            }
        }
    }
}

impl IntoIterator for MatrixConfig {
    type Item = MatrixCell;
    type IntoIter = MatrixIterator;

    fn into_iter(self) -> Self::IntoIter {
        let combinations = self.generate_combinations();
        MatrixIterator {
            config: self,
            combinations,
            index: 0,
        }
    }
}

impl Iterator for MatrixIterator {
    type Item = MatrixCell;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.combinations.len() {
            return None;
        }

        let current_index = self.index;
        let combo = self.combinations[current_index].clone();
        let name = self.config.generate_cell_name(&combo, current_index);
        let cell = MatrixCell {
            index: current_index,
            values: combo,
            name,
        };
        self.index += 1;
        Some(cell)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.combinations.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_single_axis() {
        let config = MatrixConfig {
            axes: vec![MatrixAxis::Values {
                name: "os".to_string(),
                values: vec!["linux".to_string(), "macos".to_string()],
            }],
            exclude: Vec::new(),
            agent: None,
            name_template: None,
        };

        let cells = config.generate_cells();
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].values["os"], "linux");
        assert_eq!(cells[1].values["os"], "macos");
    }

    #[test]
    fn test_matrix_two_axes() {
        let config = MatrixConfig {
            axes: vec![
                MatrixAxis::Values {
                    name: "os".to_string(),
                    values: vec!["linux".to_string(), "macos".to_string()],
                },
                MatrixAxis::Values {
                    name: "arch".to_string(),
                    values: vec!["amd64".to_string(), "arm64".to_string()],
                },
            ],
            exclude: Vec::new(),
            agent: None,
            name_template: None,
        };

        let cells = config.generate_cells();
        assert_eq!(cells.len(), 4);
    }

    #[test]
    fn test_matrix_with_exclusion() {
        let config = MatrixConfig {
            axes: vec![
                MatrixAxis::Values {
                    name: "os".to_string(),
                    values: vec!["linux".to_string(), "macos".to_string()],
                },
                MatrixAxis::Values {
                    name: "arch".to_string(),
                    values: vec!["amd64".to_string(), "arm64".to_string()],
                },
            ],
            exclude: vec![MatrixExclude {
                axes: HashMap::from([
                    ("os".to_string(), "macos".to_string()),
                    ("arch".to_string(), "arm64".to_string()),
                ]),
                reason: Some("Not supported".to_string()),
            }],
            agent: None,
            name_template: None,
        };

        let cells = config.generate_cells();
        assert_eq!(cells.len(), 3); // 4 total - 1 excluded
    }

    #[test]
    fn test_matrix_range() {
        let config = MatrixConfig {
            axes: vec![MatrixAxis::Range {
                name: "version".to_string(),
                start: 1,
                end: 3,
                step: 1,
            }],
            exclude: Vec::new(),
            agent: None,
            name_template: None,
        };

        let cells = config.generate_cells();
        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].values["version"], "1");
        assert_eq!(cells[1].values["version"], "2");
        assert_eq!(cells[2].values["version"], "3");
    }

    #[test]
    fn test_matrix_validation_empty_axes() {
        let config = MatrixConfig {
            axes: Vec::new(),
            exclude: Vec::new(),
            agent: None,
            name_template: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_matrix_validation_empty_values() {
        let config = MatrixConfig {
            axes: vec![MatrixAxis::Values {
                name: "os".to_string(),
                values: Vec::new(),
            }],
            exclude: Vec::new(),
            agent: None,
            name_template: None,
        };
        assert!(config.validate().is_err());
    }
}
