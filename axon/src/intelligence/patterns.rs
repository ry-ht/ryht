//! Pattern analysis and extraction

use super::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pattern {
    pub id: String,
    pub pattern_type: String,
    pub confidence: f32,
    pub description: String,
    pub occurrences: Vec<PatternOccurrence>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternOccurrence {
    pub location: String,
    pub context: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum Optimization {
    Parallelize { tasks: Vec<String> },
    Deduplicate { tasks: Vec<String> },
    Reorder { new_order: Vec<String> },
    Cache { task: String },
    Batch { operations: Vec<String> },
    Eliminate { redundant_tasks: Vec<String> },
}

/// Pattern types that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternType {
    CodeDuplication,
    PerformanceBottleneck,
    SecurityVulnerability,
    ArchitecturalSmell,
    OptimizationOpportunity,
    TestingGap,
    DocumentationMissing,
    ErrorHandlingMissing,
    ResourceLeak,
    ConcurrencyIssue,
}

impl PatternType {
    fn to_string(&self) -> String {
        match self {
            Self::CodeDuplication => "code_duplication",
            Self::PerformanceBottleneck => "performance_bottleneck",
            Self::SecurityVulnerability => "security_vulnerability",
            Self::ArchitecturalSmell => "architectural_smell",
            Self::OptimizationOpportunity => "optimization_opportunity",
            Self::TestingGap => "testing_gap",
            Self::DocumentationMissing => "documentation_missing",
            Self::ErrorHandlingMissing => "error_handling_missing",
            Self::ResourceLeak => "resource_leak",
            Self::ConcurrencyIssue => "concurrency_issue",
        }.to_string()
    }
}

pub struct PatternAnalyzer {
    detectors: Vec<Box<dyn PatternDetector>>,
    pattern_cache: HashMap<String, Vec<Pattern>>,
}

/// Trait for pattern detection strategies
trait PatternDetector: Send + Sync {
    fn detect(&self, data: &str, context: &AnalysisContext) -> Vec<Pattern>;
    fn pattern_type(&self) -> PatternType;
}

/// Context for pattern analysis
pub struct AnalysisContext {
    pub file_path: Option<String>,
    pub language: Option<String>,
    pub project_type: Option<String>,
    pub history: Vec<String>,
}

impl PatternAnalyzer {
    pub fn new() -> Self {
        let detectors: Vec<Box<dyn PatternDetector>> = vec![
            Box::new(CodeDuplicationDetector::new()),
            Box::new(PerformanceBottleneckDetector::new()),
            Box::new(SecurityVulnerabilityDetector::new()),
            Box::new(OptimizationOpportunityDetector::new()),
            Box::new(ErrorHandlingDetector::new()),
        ];

        Self {
            detectors,
            pattern_cache: HashMap::new(),
        }
    }

    pub fn analyze_patterns(&self, data: &str) -> Vec<Pattern> {
        self.analyze_with_context(data, &AnalysisContext::default())
    }

    pub fn analyze_with_context(&self, data: &str, context: &AnalysisContext) -> Vec<Pattern> {
        // Check cache first
        let cache_key = self.generate_cache_key(data);
        if let Some(cached) = self.pattern_cache.get(&cache_key) {
            return cached.clone();
        }

        let mut all_patterns = Vec::new();

        // Run all detectors
        for detector in &self.detectors {
            let patterns = detector.detect(data, context);
            all_patterns.extend(patterns);
        }

        // Sort by confidence
        all_patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        // Deduplicate similar patterns
        all_patterns = self.deduplicate_patterns(all_patterns);

        all_patterns
    }

    fn generate_cache_key(&self, data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn deduplicate_patterns(&self, patterns: Vec<Pattern>) -> Vec<Pattern> {
        let mut unique_patterns = Vec::new();
        let mut seen_ids = HashSet::new();

        for pattern in patterns {
            let key = format!("{}:{}", pattern.pattern_type, pattern.id);
            if seen_ids.insert(key) {
                unique_patterns.push(pattern);
            }
        }

        unique_patterns
    }

    pub fn suggest_optimizations(&self, patterns: &[Pattern]) -> Vec<Optimization> {
        let mut optimizations = Vec::new();

        for pattern in patterns {
            match pattern.pattern_type.as_str() {
                "performance_bottleneck" => {
                    if let Some(tasks) = pattern.metadata.get("affected_tasks") {
                        if let Some(tasks_array) = tasks.as_array() {
                            let task_ids: Vec<String> = tasks_array
                                .iter()
                                .filter_map(|v| v.as_str())
                                .map(String::from)
                                .collect();

                            if !task_ids.is_empty() {
                                optimizations.push(Optimization::Parallelize { tasks: task_ids });
                            }
                        }
                    }
                }
                "code_duplication" => {
                    if let Some(duplicates) = pattern.metadata.get("duplicate_blocks") {
                        if let Some(dup_array) = duplicates.as_array() {
                            let dup_tasks: Vec<String> = dup_array
                                .iter()
                                .filter_map(|v| v.as_str())
                                .map(String::from)
                                .collect();

                            if dup_tasks.len() > 1 {
                                optimizations.push(Optimization::Deduplicate { tasks: dup_tasks });
                            }
                        }
                    }
                }
                "optimization_opportunity" => {
                    if let Some(opt_type) = pattern.metadata.get("optimization_type") {
                        match opt_type.as_str() {
                            Some("cache") => {
                                if let Some(task) = pattern.metadata.get("cacheable_task") {
                                    if let Some(task_id) = task.as_str() {
                                        optimizations.push(Optimization::Cache {
                                            task: task_id.to_string(),
                                        });
                                    }
                                }
                            }
                            Some("batch") => {
                                if let Some(ops) = pattern.metadata.get("batchable_operations") {
                                    if let Some(ops_array) = ops.as_array() {
                                        let operations: Vec<String> = ops_array
                                            .iter()
                                            .filter_map(|v| v.as_str())
                                            .map(String::from)
                                            .collect();

                                        if operations.len() > 1 {
                                            optimizations.push(Optimization::Batch { operations });
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        optimizations
    }
}

impl Default for PatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AnalysisContext {
    fn default() -> Self {
        Self {
            file_path: None,
            language: None,
            project_type: None,
            history: Vec::new(),
        }
    }
}

// Pattern Detector Implementations

struct CodeDuplicationDetector {
    min_duplicate_lines: usize,
}

impl CodeDuplicationDetector {
    fn new() -> Self {
        Self {
            min_duplicate_lines: 5,
        }
    }
}

impl PatternDetector for CodeDuplicationDetector {
    fn detect(&self, data: &str, _context: &AnalysisContext) -> Vec<Pattern> {
        let mut patterns = Vec::new();
        let lines: Vec<&str> = data.lines().collect();
        let mut duplicate_blocks = HashMap::new();

        // Simple duplicate detection - check for repeated blocks
        for window_size in self.min_duplicate_lines..lines.len().min(50) {
            for i in 0..lines.len().saturating_sub(window_size) {
                let block: Vec<&str> = lines[i..i + window_size].to_vec();
                let block_str = block.join("\n");

                for j in i + window_size..lines.len().saturating_sub(window_size) {
                    let compare_block: Vec<&str> = lines[j..j + window_size].to_vec();
                    let compare_str = compare_block.join("\n");

                    if block_str == compare_str && !block_str.trim().is_empty() {
                        duplicate_blocks
                            .entry(block_str.clone())
                            .or_insert_with(Vec::new)
                            .push((i, j));
                    }
                }
            }
        }

        // Create patterns for detected duplicates
        for (block, locations) in duplicate_blocks.iter() {
            if locations.len() >= 2 {
                let mut metadata = HashMap::new();
                metadata.insert(
                    "duplicate_blocks".to_string(),
                    serde_json::json!(locations.iter().map(|(i, j)| format!("lines {}-{} and {}-{}",
                        i, i + self.min_duplicate_lines, j, j + self.min_duplicate_lines)).collect::<Vec<_>>())
                );

                patterns.push(Pattern {
                    id: uuid::Uuid::new_v4().to_string(),
                    pattern_type: PatternType::CodeDuplication.to_string(),
                    confidence: 0.9,
                    description: format!("Code duplication detected: {} occurrences", locations.len() + 1),
                    occurrences: vec![],
                    metadata,
                });
            }
        }

        patterns
    }

    fn pattern_type(&self) -> PatternType {
        PatternType::CodeDuplication
    }
}

struct PerformanceBottleneckDetector;

impl PerformanceBottleneckDetector {
    fn new() -> Self {
        Self
    }
}

impl PatternDetector for PerformanceBottleneckDetector {
    fn detect(&self, data: &str, _context: &AnalysisContext) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Detect nested loops
        let nested_loop_regex = Regex::new(r"(for|while|loop)\s*.*\{[^}]*(for|while|loop)").unwrap();
        if nested_loop_regex.is_match(data) {
            patterns.push(Pattern {
                id: uuid::Uuid::new_v4().to_string(),
                pattern_type: PatternType::PerformanceBottleneck.to_string(),
                confidence: 0.7,
                description: "Nested loops detected - potential O(n²) complexity".to_string(),
                occurrences: vec![],
                metadata: HashMap::new(),
            });
        }

        // Detect synchronous operations that could be async
        let sync_ops = ["sleep", "thread::sleep", "blocking", ".wait()"];
        for op in &sync_ops {
            if data.contains(op) {
                let mut metadata = HashMap::new();
                metadata.insert("blocking_operation".to_string(), serde_json::json!(op));

                patterns.push(Pattern {
                    id: uuid::Uuid::new_v4().to_string(),
                    pattern_type: PatternType::PerformanceBottleneck.to_string(),
                    confidence: 0.6,
                    description: format!("Blocking operation '{}' detected", op),
                    occurrences: vec![],
                    metadata,
                });
            }
        }

        // Detect large allocations in loops
        if data.contains("Vec::new") && (data.contains("for") || data.contains("while")) {
            patterns.push(Pattern {
                id: uuid::Uuid::new_v4().to_string(),
                pattern_type: PatternType::PerformanceBottleneck.to_string(),
                confidence: 0.5,
                description: "Vector allocation inside loop - consider pre-allocation".to_string(),
                occurrences: vec![],
                metadata: HashMap::new(),
            });
        }

        patterns
    }

    fn pattern_type(&self) -> PatternType {
        PatternType::PerformanceBottleneck
    }
}

struct SecurityVulnerabilityDetector;

impl SecurityVulnerabilityDetector {
    fn new() -> Self {
        Self
    }
}

impl PatternDetector for SecurityVulnerabilityDetector {
    fn detect(&self, data: &str, _context: &AnalysisContext) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Detect unsafe operations
        if data.contains("unsafe {") || data.contains("unsafe fn") {
            patterns.push(Pattern {
                id: uuid::Uuid::new_v4().to_string(),
                pattern_type: PatternType::SecurityVulnerability.to_string(),
                confidence: 0.8,
                description: "Unsafe code block detected".to_string(),
                occurrences: vec![],
                metadata: HashMap::new(),
            });
        }

        // Detect potential SQL injection
        let sql_patterns = ["format!", "concat!", "push_str"];
        let sql_keywords = ["SELECT", "INSERT", "UPDATE", "DELETE", "DROP"];

        for sql_pattern in &sql_patterns {
            if data.contains(sql_pattern) {
                for keyword in &sql_keywords {
                    if data.contains(keyword) {
                        patterns.push(Pattern {
                            id: uuid::Uuid::new_v4().to_string(),
                            pattern_type: PatternType::SecurityVulnerability.to_string(),
                            confidence: 0.7,
                            description: "Potential SQL injection vulnerability".to_string(),
                            occurrences: vec![],
                            metadata: HashMap::new(),
                        });
                        break;
                    }
                }
            }
        }

        // Detect hardcoded secrets
        let secret_patterns = [
            r"(api[_-]?key|apikey)",
            r"(secret|token|password|passwd|pwd)",
            r"(aws[_-]?access[_-]?key)",
        ];

        for pattern in &secret_patterns {
            let regex = Regex::new(&format!(r#"(?i){}\s*=\s*["'][^"']+"#, pattern)).unwrap();
            if regex.is_match(data) {
                patterns.push(Pattern {
                    id: uuid::Uuid::new_v4().to_string(),
                    pattern_type: PatternType::SecurityVulnerability.to_string(),
                    confidence: 0.9,
                    description: "Potential hardcoded secret detected".to_string(),
                    occurrences: vec![],
                    metadata: HashMap::new(),
                });
            }
        }

        patterns
    }

    fn pattern_type(&self) -> PatternType {
        PatternType::SecurityVulnerability
    }
}

struct OptimizationOpportunityDetector;

impl OptimizationOpportunityDetector {
    fn new() -> Self {
        Self
    }
}

impl PatternDetector for OptimizationOpportunityDetector {
    fn detect(&self, data: &str, _context: &AnalysisContext) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Detect multiple consecutive similar operations that could be batched
        let operation_patterns = [
            (r"db\.(insert|update|delete)", "database operations"),
            (r"file\.(write|read)", "file operations"),
            (r"http\.(get|post|put|delete)", "HTTP requests"),
        ];

        for (pattern, op_type) in &operation_patterns {
            let regex = Regex::new(pattern).unwrap();
            let matches: Vec<_> = regex.find_iter(data).collect();

            if matches.len() > 3 {
                let mut metadata = HashMap::new();
                metadata.insert("optimization_type".to_string(), serde_json::json!("batch"));
                metadata.insert("batchable_operations".to_string(),
                    serde_json::json!(matches.iter().map(|m| m.as_str()).collect::<Vec<_>>()));

                patterns.push(Pattern {
                    id: uuid::Uuid::new_v4().to_string(),
                    pattern_type: PatternType::OptimizationOpportunity.to_string(),
                    confidence: 0.7,
                    description: format!("Multiple {} could be batched", op_type),
                    occurrences: vec![],
                    metadata,
                });
            }
        }

        // Detect repeated expensive computations that could be cached
        let expensive_ops = ["calculate", "compute", "process", "transform"];
        for op in &expensive_ops {
            let pattern = format!(r"{}[_a-z]*\([^)]*\)", op);
            let regex = Regex::new(&pattern).unwrap();
            let matches: Vec<_> = regex.find_iter(data).collect();

            let mut call_counts = HashMap::new();
            for m in matches {
                *call_counts.entry(m.as_str()).or_insert(0) += 1;
            }

            for (call, count) in call_counts {
                if count > 2 {
                    let mut metadata = HashMap::new();
                    metadata.insert("optimization_type".to_string(), serde_json::json!("cache"));
                    metadata.insert("cacheable_task".to_string(), serde_json::json!(call));

                    patterns.push(Pattern {
                        id: uuid::Uuid::new_v4().to_string(),
                        pattern_type: PatternType::OptimizationOpportunity.to_string(),
                        confidence: 0.6,
                        description: format!("Repeated computation '{}' could be cached", call),
                        occurrences: vec![],
                        metadata,
                    });
                }
            }
        }

        patterns
    }

    fn pattern_type(&self) -> PatternType {
        PatternType::OptimizationOpportunity
    }
}

struct ErrorHandlingDetector;

impl ErrorHandlingDetector {
    fn new() -> Self {
        Self
    }
}

impl PatternDetector for ErrorHandlingDetector {
    fn detect(&self, data: &str, _context: &AnalysisContext) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Detect unwrap() calls
        let unwrap_count = data.matches(".unwrap()").count();
        if unwrap_count > 0 {
            patterns.push(Pattern {
                id: uuid::Uuid::new_v4().to_string(),
                pattern_type: PatternType::ErrorHandlingMissing.to_string(),
                confidence: 0.8,
                description: format!("{} unwrap() calls found - consider proper error handling", unwrap_count),
                occurrences: vec![],
                metadata: HashMap::new(),
            });
        }

        // Detect expect() without meaningful messages
        let expect_regex = Regex::new(r#"\.expect\(["'][^"']*["']\)"#).unwrap();
        for m in expect_regex.find_iter(data) {
            let expect_str = m.as_str();
            if expect_str.contains(r#"expect("")"#) || expect_str.contains(r#"expect("error")"#) {
                patterns.push(Pattern {
                    id: uuid::Uuid::new_v4().to_string(),
                    pattern_type: PatternType::ErrorHandlingMissing.to_string(),
                    confidence: 0.6,
                    description: "expect() with non-descriptive message".to_string(),
                    occurrences: vec![],
                    metadata: HashMap::new(),
                });
                break;
            }
        }

        // Detect ignored Results
        if data.contains("let _ =") && data.contains("Result<") {
            patterns.push(Pattern {
                id: uuid::Uuid::new_v4().to_string(),
                pattern_type: PatternType::ErrorHandlingMissing.to_string(),
                confidence: 0.7,
                description: "Result type being ignored with 'let _ ='".to_string(),
                occurrences: vec![],
                metadata: HashMap::new(),
            });
        }

        patterns
    }

    fn pattern_type(&self) -> PatternType {
        PatternType::ErrorHandlingMissing
    }
}
