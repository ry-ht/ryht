// Core types for security scanning

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Security finding severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityLevel {
    Critical,  // Immediate action required
    High,      // Important to fix
    Medium,    // Should be addressed
    Low,       // Minor issue
    Info,      // Informational only
}

/// OWASP Top 10 and other vulnerability categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VulnerabilityCategory {
    // OWASP Top 10 2021
    BrokenAccessControl,           // A01:2021
    CryptographicFailures,         // A02:2021
    Injection,                     // A03:2021
    InsecureDesign,                // A04:2021
    SecurityMisconfiguration,      // A05:2021
    VulnerableComponents,          // A06:2021
    AuthenticationFailures,        // A07:2021
    DataIntegrityFailures,         // A08:2021
    LoggingMonitoringFailures,     // A09:2021
    ServerSideRequestForgery,      // A10:2021

    // Additional categories
    SecretsExposure,
    HardcodedCredentials,
    InsecureCommunication,
    PathTraversal,
    CommandInjection,
    XmlExternalEntity,
    UnvalidatedRedirects,
    InsecureDeserialization,
    ResourceExhaustion,
    RaceCondition,
}

/// Specific vulnerability types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VulnerabilityType {
    // Secrets
    AwsAccessKey,
    AwsSecretKey,
    GithubToken,
    PrivateKey,
    JwtSecret,
    ApiKey,
    Password,

    // Injection
    SqlInjection,
    NoSqlInjection,
    CommandInjection,
    LdapInjection,
    XpathInjection,

    // XSS
    ReflectedXss,
    StoredXss,
    DomBasedXss,

    // Crypto
    WeakCrypto,
    HardcodedIv,
    InsecureRandom,

    // Authentication
    WeakPasswordPolicy,
    MissingAuthentication,
    SessionFixation,

    // Authorization
    MissingAuthorization,
    InsecureDirectObjectReference,

    // Configuration
    DebugEnabled,
    DefaultCredentials,
    ExcessivePermissions,

    // Dependencies
    OutdatedDependency,
    KnownVulnerableDependency,

    // Other
    PathTraversal,
    Ssrf,
    Xxe,
    Csrf,
    OpenRedirect,
    InsecureDeserialization,
    UnhandledError,
}

/// A security finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub level: SecurityLevel,
    pub category: VulnerabilityCategory,
    pub vuln_type: VulnerabilityType,
    pub title: String,
    pub description: String,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub column_number: Option<usize>,
    pub code_snippet: Option<String>,
    pub cwe_id: Option<String>,       // Common Weakness Enumeration ID
    pub cve_ids: Vec<String>,         // CVE identifiers if applicable
    pub owasp_category: Option<String>, // e.g., "A03:2021"
    pub remediation: String,
    pub references: Vec<String>,
    pub confidence: f32,              // 0.0-1.0, how confident we are
    pub metadata: HashMap<String, String>,
}

/// Scan result containing all findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub total_files: usize,
    pub scanned_files: usize,
    pub scan_duration_ms: u64,
    pub findings: Vec<SecurityFinding>,
    pub stats: SecurityStats,
}

/// Statistics about findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityStats {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub by_category: HashMap<VulnerabilityCategory, usize>,
    pub by_type: HashMap<VulnerabilityType, usize>,
    pub false_positive_rate: f32,
}

/// Scan options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    pub scan_secrets: bool,
    pub scan_owasp: bool,
    pub scan_dependencies: bool,
    pub scan_patterns: bool,
    pub excluded_paths: Vec<String>,
    pub max_file_size_kb: usize,
    pub confidence_threshold: f32,  // Only report findings above this confidence
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            scan_secrets: true,
            scan_owasp: true,
            scan_dependencies: true,
            scan_patterns: true,
            excluded_paths: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
            max_file_size_kb: 1024, // 1MB
            confidence_threshold: 0.7,
        }
    }
}

/// Pattern for detecting secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPattern {
    pub name: String,
    pub pattern: String,  // Regex pattern
    pub vuln_type: VulnerabilityType,
    pub level: SecurityLevel,
    pub description: String,
    pub remediation: String,
}

impl SecurityLevel {
    pub fn to_string(&self) -> &'static str {
        match self {
            SecurityLevel::Critical => "critical",
            SecurityLevel::High => "high",
            SecurityLevel::Medium => "medium",
            SecurityLevel::Low => "low",
            SecurityLevel::Info => "info",
        }
    }
}

impl VulnerabilityCategory {
    pub fn owasp_id(&self) -> Option<&'static str> {
        match self {
            VulnerabilityCategory::BrokenAccessControl => Some("A01:2021"),
            VulnerabilityCategory::CryptographicFailures => Some("A02:2021"),
            VulnerabilityCategory::Injection => Some("A03:2021"),
            VulnerabilityCategory::InsecureDesign => Some("A04:2021"),
            VulnerabilityCategory::SecurityMisconfiguration => Some("A05:2021"),
            VulnerabilityCategory::VulnerableComponents => Some("A06:2021"),
            VulnerabilityCategory::AuthenticationFailures => Some("A07:2021"),
            VulnerabilityCategory::DataIntegrityFailures => Some("A08:2021"),
            VulnerabilityCategory::LoggingMonitoringFailures => Some("A09:2021"),
            VulnerabilityCategory::ServerSideRequestForgery => Some("A10:2021"),
            _ => None,
        }
    }
}

impl ScanResult {
    pub fn new() -> Self {
        Self {
            total_files: 0,
            scanned_files: 0,
            scan_duration_ms: 0,
            findings: Vec::new(),
            stats: SecurityStats::default(),
        }
    }

    pub fn add_finding(&mut self, finding: SecurityFinding) {
        // Update stats
        match finding.level {
            SecurityLevel::Critical => self.stats.critical += 1,
            SecurityLevel::High => self.stats.high += 1,
            SecurityLevel::Medium => self.stats.medium += 1,
            SecurityLevel::Low => self.stats.low += 1,
            SecurityLevel::Info => self.stats.info += 1,
        }

        *self.stats.by_category.entry(finding.category.clone()).or_insert(0) += 1;
        *self.stats.by_type.entry(finding.vuln_type.clone()).or_insert(0) += 1;

        self.findings.push(finding);
    }
}

impl Default for SecurityStats {
    fn default() -> Self {
        Self {
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
            by_category: HashMap::new(),
            by_type: HashMap::new(),
            false_positive_rate: 0.0,
        }
    }
}
