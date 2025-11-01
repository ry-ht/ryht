//! Token counting and efficiency measurement
//!
//! Provides utilities for:
//! - Estimating token counts from text
//! - Comparing traditional vs Cortex approaches
//! - Calculating efficiency savings
//! - Generating detailed efficiency reports

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Token counter for measuring efficiency
pub struct TokenCounter {
    measurements: Vec<TokenMeasurement>,
    scenarios: HashMap<String, ScenarioStats>,
}

/// A single token measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMeasurement {
    pub scenario: String,
    pub traditional_tokens: usize,
    pub cortex_tokens: usize,
    pub savings_tokens: usize,
    pub savings_percent: f64,
    pub category: String,
}

/// Statistics for a scenario category
#[derive(Debug, Clone, Default)]
struct ScenarioStats {
    total_traditional: usize,
    total_cortex: usize,
    count: usize,
}

impl TokenCounter {
    /// Create a new token counter
    pub fn new() -> Self {
        Self {
            measurements: Vec::new(),
            scenarios: HashMap::new(),
        }
    }

    /// Add a measurement
    pub fn add_measurement(
        &mut self,
        scenario: impl Into<String>,
        category: impl Into<String>,
        traditional_tokens: usize,
        cortex_tokens: usize,
    ) {
        let scenario = scenario.into();
        let category = category.into();

        let savings_tokens = traditional_tokens.saturating_sub(cortex_tokens);
        let savings_percent = if traditional_tokens > 0 {
            100.0 * savings_tokens as f64 / traditional_tokens as f64
        } else {
            0.0
        };

        let measurement = TokenMeasurement {
            scenario: scenario.clone(),
            traditional_tokens,
            cortex_tokens,
            savings_tokens,
            savings_percent,
            category: category.clone(),
        };

        self.measurements.push(measurement);

        // Update category stats
        let stats = self.scenarios.entry(category).or_default();
        stats.total_traditional += traditional_tokens;
        stats.total_cortex += cortex_tokens;
        stats.count += 1;
    }

    /// Get all measurements
    pub fn measurements(&self) -> &[TokenMeasurement] {
        &self.measurements
    }

    /// Calculate total savings
    pub fn total_savings(&self) -> TokenComparison {
        let total_traditional: usize = self.measurements.iter()
            .map(|m| m.traditional_tokens)
            .sum();

        let total_cortex: usize = self.measurements.iter()
            .map(|m| m.cortex_tokens)
            .sum();

        TokenComparison::new(total_traditional, total_cortex)
    }

    /// Generate efficiency report
    pub fn generate_report(&self) -> EfficiencyReport {
        EfficiencyReport {
            measurements: self.measurements.clone(),
            total: self.total_savings(),
            by_category: self.category_summaries(),
        }
    }

    /// Get summaries by category
    fn category_summaries(&self) -> HashMap<String, TokenComparison> {
        self.scenarios
            .iter()
            .map(|(category, stats)| {
                (
                    category.clone(),
                    TokenComparison::new(stats.total_traditional, stats.total_cortex),
                )
            })
            .collect()
    }

    /// Print summary to console
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(100));
        println!("{:^100}", "TOKEN EFFICIENCY REPORT");
        println!("{}", "=".repeat(100));

        // Individual measurements
        println!("\n{:<40} {:<20} {:>12} {:>12} {:>12}",
            "Scenario", "Category", "Traditional", "Cortex", "Savings %");
        println!("{}", "-".repeat(100));

        for m in &self.measurements {
            println!(
                "{:<40} {:<20} {:>12} {:>12} {:>11.1}%",
                truncate_str(&m.scenario, 40),
                truncate_str(&m.category, 20),
                m.traditional_tokens,
                m.cortex_tokens,
                m.savings_percent
            );
        }

        // Category summaries
        if !self.scenarios.is_empty() {
            println!("\n{}", "=".repeat(100));
            println!("CATEGORY SUMMARIES");
            println!("{}", "-".repeat(100));
            println!("{:<40} {:>12} {:>12} {:>12} {:>12}",
                "Category", "Traditional", "Cortex", "Saved", "Savings %");
            println!("{}", "-".repeat(100));

            for (category, stats) in &self.scenarios {
                let savings = stats.total_traditional.saturating_sub(stats.total_cortex);
                let savings_pct = if stats.total_traditional > 0 {
                    100.0 * savings as f64 / stats.total_traditional as f64
                } else {
                    0.0
                };

                println!(
                    "{:<40} {:>12} {:>12} {:>12} {:>11.1}%",
                    truncate_str(category, 40),
                    stats.total_traditional,
                    stats.total_cortex,
                    savings,
                    savings_pct
                );
            }
        }

        // Overall summary
        let total = self.total_savings();
        println!("\n{}", "=".repeat(100));
        println!("OVERALL SUMMARY");
        println!("{}", "-".repeat(100));
        println!("Total Traditional Tokens:  {:>12}", total.traditional_tokens);
        println!("Total Cortex Tokens:       {:>12}", total.cortex_tokens);
        println!("Total Tokens Saved:        {:>12}", total.savings_tokens);
        println!("Overall Savings:           {:>11.1}%", total.savings_percent);
        println!("{}", "=".repeat(100));
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison between traditional and Cortex token usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenComparison {
    pub traditional_tokens: usize,
    pub cortex_tokens: usize,
    pub savings_tokens: usize,
    pub savings_percent: f64,
}

impl TokenComparison {
    pub fn new(traditional_tokens: usize, cortex_tokens: usize) -> Self {
        let savings_tokens = traditional_tokens.saturating_sub(cortex_tokens);
        let savings_percent = if traditional_tokens > 0 {
            100.0 * savings_tokens as f64 / traditional_tokens as f64
        } else {
            0.0
        };

        Self {
            traditional_tokens,
            cortex_tokens,
            savings_tokens,
            savings_percent,
        }
    }

    /// Check if savings meet a threshold
    pub fn meets_threshold(&self, min_percent: f64) -> bool {
        self.savings_percent >= min_percent
    }
}

/// Detailed efficiency report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyReport {
    pub measurements: Vec<TokenMeasurement>,
    pub total: TokenComparison,
    pub by_category: HashMap<String, TokenComparison>,
}

impl EfficiencyReport {
    /// Export to JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Save to file
    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.to_json())
    }

    /// Print markdown report
    pub fn print_markdown(&self) {
        println!("# Token Efficiency Report\n");

        println!("## Overall Summary\n");
        println!("| Metric | Value |");
        println!("|--------|------:|");
        println!("| Traditional Tokens | {:,} |", self.total.traditional_tokens);
        println!("| Cortex Tokens | {:,} |", self.total.cortex_tokens);
        println!("| Tokens Saved | {:,} |", self.total.savings_tokens);
        println!("| Savings Percentage | {:.1}% |\n", self.total.savings_percent);

        if !self.by_category.is_empty() {
            println!("## By Category\n");
            println!("| Category | Traditional | Cortex | Saved | Savings % |");
            println!("|----------|------------:|-------:|------:|----------:|");

            let mut categories: Vec<_> = self.by_category.iter().collect();
            categories.sort_by_key(|(name, _)| *name);

            for (category, comparison) in categories {
                println!(
                    "| {} | {:,} | {:,} | {:,} | {:.1}% |",
                    category,
                    comparison.traditional_tokens,
                    comparison.cortex_tokens,
                    comparison.savings_tokens,
                    comparison.savings_percent
                );
            }
            println!();
        }

        println!("## Detailed Measurements\n");
        println!("| Scenario | Category | Traditional | Cortex | Savings % |");
        println!("|----------|----------|------------:|-------:|----------:|");

        for m in &self.measurements {
            println!(
                "| {} | {} | {:,} | {:,} | {:.1}% |",
                m.scenario,
                m.category,
                m.traditional_tokens,
                m.cortex_tokens,
                m.savings_percent
            );
        }
    }
}

/// Estimate token count from text
pub fn estimate_tokens(text: &str) -> usize {
    // Simple estimation: ~4 characters per token on average
    // This is a rough approximation; real tokenization would use a proper tokenizer
    let char_count = text.chars().count();
    let word_count = text.split_whitespace().count();

    // Average of character-based and word-based estimates
    ((char_count / 4) + (word_count * 1.3 as usize)) / 2
}

/// Estimate tokens from a file
pub fn estimate_tokens_from_file(content: &str) -> usize {
    estimate_tokens(content)
}

/// Estimate tokens for reading multiple files
pub fn estimate_tokens_for_files(files: &[&str]) -> usize {
    files.iter().map(|f| estimate_tokens(f)).sum()
}

/// Common scenarios and their token estimates

/// Estimate tokens for grep operation
pub fn estimate_grep_tokens(pattern: &str, num_results: usize, avg_line_length: usize) -> usize {
    let query_tokens = estimate_tokens(pattern);
    let result_tokens = num_results * avg_line_length / 4; // 4 chars per token
    query_tokens + result_tokens
}

/// Estimate tokens for reading a file
pub fn estimate_read_file_tokens(file_size_bytes: usize) -> usize {
    // Assume ~1 byte = ~0.25 tokens on average
    file_size_bytes / 4
}

/// Estimate tokens for semantic search query
pub fn estimate_semantic_search_tokens(query: &str, num_results: usize) -> usize {
    let query_tokens = estimate_tokens(query);
    // Results are typically compact summaries
    let results_tokens = num_results * 20; // ~20 tokens per result
    query_tokens + results_tokens
}

/// Estimate tokens for dependency graph
pub fn estimate_dependency_tokens(num_nodes: usize, num_edges: usize) -> usize {
    // Each node: ~15 tokens (id, name, type)
    // Each edge: ~10 tokens (from, to, type)
    num_nodes * 15 + num_edges * 10
}

/// Estimate tokens for code unit
pub fn estimate_code_unit_tokens(unit_name: &str, unit_type: &str, has_body: bool) -> usize {
    let base = estimate_tokens(unit_name) + estimate_tokens(unit_type);
    if has_body {
        base + 50 // Body summary
    } else {
        base + 5 // Just signature
    }
}

// Helper functions

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counter() {
        let mut counter = TokenCounter::new();

        counter.add_measurement("Find functions", "Search", 30000, 100);
        counter.add_measurement("Modify code", "Manipulation", 1200, 100);
        counter.add_measurement("Analyze deps", "Analysis", 5000, 200);

        let total = counter.total_savings();
        assert_eq!(total.traditional_tokens, 36200);
        assert_eq!(total.cortex_tokens, 400);
        assert!(total.savings_percent > 98.0);
    }

    #[test]
    fn test_token_estimation() {
        let text = "Hello world this is a test";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        assert!(tokens < 20); // Should be reasonable
    }

    #[test]
    fn test_token_comparison() {
        let comp = TokenComparison::new(1000, 100);
        assert_eq!(comp.savings_tokens, 900);
        assert_eq!(comp.savings_percent, 90.0);
        assert!(comp.meets_threshold(75.0));
        assert!(!comp.meets_threshold(95.0));
    }
}
