//! Researcher Agent Implementation
//!
//! The Researcher Agent specializes in information gathering and analysis.
//! It provides capabilities for:
//! - Information retrieval and synthesis
//! - Trend analysis
//! - Technology research
//! - Fact checking
//! - Integration with CortexBridge for semantic search

use super::*;
use crate::cortex_bridge::{
    CortexBridge, SearchFilters, WorkspaceId, UnitFilters,
};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};

/// Researcher agent for information gathering and analysis
pub struct ResearcherAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,

    // Research-specific configuration
    search_strategies: Vec<SearchStrategy>,
    information_sources: Vec<InformationSource>,

    // Cortex integration (optional for backward compatibility)
    cortex: Option<Arc<CortexBridge>>,
    workspace_id: Option<WorkspaceId>,
}

/// Search strategy for information retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchStrategy {
    /// Broad keyword-based search
    BroadKeyword,

    /// Semantic similarity search
    Semantic,

    /// Citation and reference tracking
    Citation,

    /// Trending topics analysis
    TrendingTopics,

    /// Deep dive into specific domain
    DomainExpert,
}

/// Information source type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InformationSource {
    /// Code repositories
    CodeRepository,

    /// Documentation
    Documentation,

    /// Academic papers
    AcademicPapers,

    /// Technical blogs
    TechnicalBlogs,

    /// Community forums
    CommunityForums,

    /// Official specifications
    Specifications,

    /// Knowledge base
    KnowledgeBase,
}

/// Research query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuery {
    /// Main query string
    pub query: String,

    /// Query type
    pub query_type: QueryType,

    /// Scope of research
    pub scope: ResearchScope,

    /// Maximum results to return
    pub max_results: usize,

    /// Time range for research
    pub time_range: Option<TimeRange>,

    /// Quality threshold (0.0 to 1.0)
    pub quality_threshold: f32,
}

/// Type of research query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    /// General information retrieval
    General,

    /// Technology comparison
    TechnologyComparison,

    /// Best practices research
    BestPractices,

    /// Trend analysis
    TrendAnalysis,

    /// Fact verification
    FactChecking,

    /// Problem solving
    ProblemSolving,
}

/// Scope of research
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResearchScope {
    /// Local codebase only
    Local,

    /// Organization-wide
    Organization,

    /// Public knowledge
    Public,

    /// Combined local and public
    Combined,
}

/// Time range for research
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Research report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReport {
    /// Query that generated this report
    pub query: String,

    /// Summary of findings
    pub summary: String,

    /// Key findings
    pub key_findings: Vec<Finding>,

    /// Sources consulted
    pub sources: Vec<Source>,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,

    /// Recommendations
    pub recommendations: Vec<String>,

    /// Related topics
    pub related_topics: Vec<String>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Individual finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Title of the finding
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Relevance score (0.0 to 1.0)
    pub relevance: f32,

    /// Confidence in this finding (0.0 to 1.0)
    pub confidence: f32,

    /// Supporting sources
    pub sources: Vec<String>,

    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Information source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Source title
    pub title: String,

    /// Source URL or identifier
    pub url: String,

    /// Source type
    pub source_type: InformationSource,

    /// Quality score (0.0 to 1.0)
    pub quality_score: f32,

    /// Last accessed/updated
    pub last_accessed: DateTime<Utc>,
}

/// Trend analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Topic being analyzed
    pub topic: String,

    /// Trend direction
    pub direction: TrendDirection,

    /// Strength of trend (0.0 to 1.0)
    pub strength: f32,

    /// Time series data points
    pub data_points: Vec<TrendDataPoint>,

    /// Predictions
    pub predictions: Vec<String>,

    /// Analysis summary
    pub summary: String,
}

/// Direction of a trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Rising,
    Falling,
    Stable,
    Volatile,
}

/// Data point in trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f32,
    pub label: String,
}

/// Technology comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologyComparison {
    /// Technologies being compared
    pub technologies: Vec<String>,

    /// Comparison dimensions
    pub dimensions: Vec<ComparisonDimension>,

    /// Overall recommendation
    pub recommendation: String,

    /// Detailed analysis
    pub analysis: String,
}

/// Dimension for technology comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonDimension {
    pub name: String,
    pub scores: HashMap<String, f32>,
    pub notes: String,
}

impl ResearcherAgent {
    /// Create a new researcher agent with default configuration (no Cortex)
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::InformationRetrieval);
        capabilities.insert(Capability::FactChecking);
        capabilities.insert(Capability::TrendAnalysis);
        capabilities.insert(Capability::TechnologyResearch);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            search_strategies: vec![
                SearchStrategy::BroadKeyword,
                SearchStrategy::Semantic,
                SearchStrategy::TrendingTopics,
            ],
            information_sources: vec![
                InformationSource::CodeRepository,
                InformationSource::Documentation,
                InformationSource::KnowledgeBase,
            ],
            cortex: None,
            workspace_id: None,
        }
    }

    /// Create a new researcher agent with Cortex integration
    pub fn with_cortex(name: String, cortex: Arc<CortexBridge>, workspace_id: WorkspaceId) -> Self {
        let mut agent = Self::new(name);
        agent.cortex = Some(cortex);
        agent.workspace_id = Some(workspace_id);
        agent
    }

    /// Create researcher agent with custom strategies
    pub fn with_strategies(
        name: String,
        strategies: Vec<SearchStrategy>,
        sources: Vec<InformationSource>,
    ) -> Self {
        let mut agent = Self::new(name);
        agent.search_strategies = strategies;
        agent.information_sources = sources;
        agent
    }

    /// Conduct research on a topic (sync version, no Cortex)
    ///
    /// This is a synchronous version for backward compatibility.
    /// For Cortex integration, use `research_async`.
    pub fn research(&self, query: ResearchQuery) -> Result<ResearchReport> {
        info!("Starting research for query: {} (sync)", query.query);

        // Select appropriate search strategy
        let strategy = self.select_strategy(&query);

        // Use basic findings without Cortex
        let raw_findings = vec![RawFinding {
            content: format!("Finding for: {}", query.query),
            source: "local".to_string(),
            relevance: 0.7,
        }];

        // Filter and validate information
        let validated_findings = self.validate_information(raw_findings, query.quality_threshold);

        // Analyze and synthesize findings
        let key_findings = self.synthesize_findings(validated_findings);

        // Calculate confidence before moving key_findings
        let confidence = self.calculate_confidence(&key_findings);

        // Generate summary before moving key_findings
        let summary = self.generate_summary(&key_findings);

        // Generate recommendations
        let recommendations = self.generate_recommendations(&key_findings);

        // Identify related topics
        let related_topics = self.identify_related_topics(&query, &key_findings);

        Ok(ResearchReport {
            query: query.query.clone(),
            summary,
            key_findings,
            sources: self.get_consulted_sources(),
            confidence,
            recommendations,
            related_topics,
            created_at: Utc::now(),
        })
    }

    /// Conduct research on a topic with Cortex integration (async version)
    pub async fn research_async(&self, query: ResearchQuery) -> Result<ResearchReport> {
        info!("Starting research for query: {}", query.query);

        // Select appropriate search strategy
        let strategy = self.select_strategy(&query);

        // Gather information from various sources (now async)
        let raw_findings = self.gather_information(&query, &strategy).await?;

        // Filter and validate information
        let validated_findings = self.validate_information(raw_findings, query.quality_threshold);

        // Analyze and synthesize findings
        let key_findings = self.synthesize_findings(validated_findings);

        // Calculate confidence before moving key_findings
        let confidence = self.calculate_confidence(&key_findings);

        // Generate summary before moving key_findings
        let summary = self.generate_summary(&key_findings);

        // Generate recommendations
        let recommendations = self.generate_recommendations(&key_findings);

        // Identify related topics
        let related_topics = self.identify_related_topics(&query, &key_findings);

        info!("Research completed with {} findings", key_findings.len());

        Ok(ResearchReport {
            query: query.query.clone(),
            summary,
            key_findings,
            sources: self.get_consulted_sources(),
            confidence,
            recommendations,
            related_topics,
            created_at: Utc::now(),
        })
    }

    /// Analyze trends for a topic
    pub fn analyze_trends(&self, topic: String, time_range: TimeRange) -> Result<TrendAnalysis> {
        // Collect historical data
        let data_points = self.collect_trend_data(&topic, &time_range);

        // Analyze trend direction
        let direction = self.determine_trend_direction(&data_points);

        // Calculate trend strength
        let strength = self.calculate_trend_strength(&data_points);

        // Generate summary before moving direction
        let summary = format!(
            "Trend for '{}' is {:?} with strength {:.2}",
            topic, direction, strength
        );

        // Generate predictions
        let predictions = self.generate_trend_predictions(&data_points, &direction);

        Ok(TrendAnalysis {
            topic: topic.clone(),
            direction,
            strength,
            data_points,
            predictions,
            summary,
        })
    }

    /// Compare technologies
    pub fn compare_technologies(
        &self,
        technologies: Vec<String>,
        dimensions: Vec<String>,
    ) -> Result<TechnologyComparison> {
        // Research each technology
        let tech_data = self.research_technologies(&technologies)?;

        // Compare across dimensions
        let comparison_dims = self.compare_dimensions(&tech_data, &dimensions);

        // Generate recommendation
        let recommendation = self.generate_technology_recommendation(&comparison_dims);

        // Create analysis before moving recommendation
        let analysis = format!(
            "Compared {} technologies across {} dimensions. {}",
            technologies.len(),
            dimensions.len(),
            recommendation
        );

        Ok(TechnologyComparison {
            technologies,
            dimensions: comparison_dims,
            recommendation,
            analysis,
        })
    }

    /// Verify facts
    pub fn check_facts(&self, claims: Vec<String>) -> Result<Vec<FactCheckResult>> {
        claims
            .into_iter()
            .map(|claim| self.verify_claim(&claim))
            .collect()
    }

    /// Get supported search strategies
    pub fn get_search_strategies(&self) -> &[SearchStrategy] {
        &self.search_strategies
    }

    /// Get supported information sources
    pub fn get_information_sources(&self) -> &[InformationSource] {
        &self.information_sources
    }

    // Private helper methods

    fn select_strategy(&self, query: &ResearchQuery) -> SearchStrategy {
        match query.query_type {
            QueryType::TrendAnalysis => SearchStrategy::TrendingTopics,
            QueryType::FactChecking => SearchStrategy::Citation,
            QueryType::TechnologyComparison => SearchStrategy::DomainExpert,
            _ => SearchStrategy::Semantic,
        }
    }

    /// Gather information using CortexBridge semantic search
    ///
    /// This method implements real integration with Cortex:
    /// 1. Uses semantic_search for finding relevant code and documentation
    /// 2. Filters results by relevance threshold
    /// 3. Applies different search strategies (BroadKeyword, Semantic, etc.)
    /// 4. Returns findings with sources and relevance scores
    ///
    /// # Arguments
    /// * `query` - Research query with parameters
    /// * `strategy` - Search strategy to apply
    ///
    /// # Returns
    /// Vector of raw findings with relevance scores
    async fn gather_information(
        &self,
        query: &ResearchQuery,
        strategy: &SearchStrategy,
    ) -> Result<Vec<RawFinding>> {
        let mut findings = Vec::new();

        info!("Gathering information for query: {}", query.query);
        debug!("Using strategy: {:?}", strategy);

        // If Cortex is available, use it for semantic search
        if let (Some(cortex), Some(workspace_id)) = (&self.cortex, &self.workspace_id) {
            // Configure search filters based on strategy
            let filters = match strategy {
                SearchStrategy::Semantic => SearchFilters {
                    types: vec![
                        "function".to_string(),
                        "class".to_string(),
                        "module".to_string(),
                        "interface".to_string(),
                    ],
                    min_relevance: query.quality_threshold,
                    ..Default::default()
                },
                SearchStrategy::BroadKeyword => SearchFilters {
                    types: vec![],
                    min_relevance: query.quality_threshold * 0.8, // Lower threshold for broad search
                    ..Default::default()
                },
                SearchStrategy::DomainExpert => SearchFilters {
                    types: vec!["class".to_string(), "interface".to_string()],
                    min_relevance: query.quality_threshold * 1.1, // Higher threshold for expert search
                    ..Default::default()
                },
                SearchStrategy::Citation => SearchFilters {
                    types: vec!["documentation".to_string(), "comment".to_string()],
                    min_relevance: query.quality_threshold,
                    ..Default::default()
                },
                SearchStrategy::TrendingTopics => SearchFilters {
                    types: vec![],
                    min_relevance: query.quality_threshold * 0.9,
                    ..Default::default()
                },
            };

            // Perform semantic search
            match cortex
                .semantic_search(&query.query, workspace_id, filters)
                .await
            {
                Ok(results) => {
                    info!("Found {} results from Cortex semantic search", results.len());

                    // Convert search results to findings
                    for result in results.into_iter().take(query.max_results) {
                        // Filter by relevance threshold
                        if result.relevance_score >= query.quality_threshold {
                            findings.push(RawFinding {
                                content: format!(
                                    "{}\n\nFile: {}\nSignature: {}\nSnippet:\n{}",
                                    result.name,
                                    result.file,
                                    result.signature,
                                    result.snippet
                                ),
                                source: result.file.clone(),
                                relevance: result.relevance_score,
                            });
                        }
                    }

                    debug!("Filtered to {} findings above threshold", findings.len());
                }
                Err(e) => {
                    warn!("Cortex semantic search failed: {}", e);
                    // Fall back to basic finding
                    findings.push(RawFinding {
                        content: format!("Finding for: {} (fallback mode)", query.query),
                        source: "fallback".to_string(),
                        relevance: 0.5,
                    });
                }
            }

            // If strategy is Semantic or DomainExpert, also search episodes for similar research
            if matches!(strategy, SearchStrategy::Semantic | SearchStrategy::DomainExpert) {
                match cortex.search_episodes(&query.query, 5).await {
                    Ok(episodes) => {
                        info!("Found {} related episodes", episodes.len());

                        for episode in episodes {
                            // Extract insights from previous research
                            if !episode.lessons_learned.is_empty() {
                                let lessons = episode.lessons_learned.join("; ");
                                findings.push(RawFinding {
                                    content: format!(
                                        "Previous research insight: {}\nSummary: {}",
                                        lessons, episode.solution_summary
                                    ),
                                    source: format!("episode:{}", episode.id),
                                    relevance: 0.8, // High relevance for learned knowledge
                                });
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to search episodes: {}", e);
                    }
                }
            }

            // For TrendingTopics strategy, get code units to analyze patterns
            if matches!(strategy, SearchStrategy::TrendingTopics) {
                match cortex
                    .get_code_units(
                        workspace_id,
                        UnitFilters {
                            unit_type: None,
                            language: None,
                            visibility: Some("public".to_string()),
                        },
                    )
                    .await
                {
                    Ok(units) => {
                        info!("Retrieved {} code units for trend analysis", units.len());

                        // Analyze unit types distribution for trends
                        let mut type_counts: HashMap<String, usize> = HashMap::new();
                        for unit in units.iter().take(100) {
                            *type_counts.entry(unit.unit_type.clone()).or_insert(0) += 1;
                        }

                        let trend_summary = type_counts
                            .iter()
                            .map(|(t, c)| format!("{}: {}", t, c))
                            .collect::<Vec<_>>()
                            .join(", ");

                        findings.push(RawFinding {
                            content: format!(
                                "Code trends analysis: {}\nTotal units analyzed: {}",
                                trend_summary,
                                units.len()
                            ),
                            source: "trend_analysis".to_string(),
                            relevance: 0.75,
                        });
                    }
                    Err(e) => {
                        debug!("Failed to get code units: {}", e);
                    }
                }
            }
        } else {
            // No Cortex - use placeholder finding
            debug!("No Cortex available, using placeholder finding");
            findings.push(RawFinding {
                content: format!("Finding for: {}", query.query),
                source: "local".to_string(),
                relevance: 0.7,
            });
        }

        if findings.is_empty() {
            warn!("No findings gathered for query: {}", query.query);
        } else {
            info!("Gathered {} findings", findings.len());
        }

        Ok(findings)
    }

    fn validate_information(&self, findings: Vec<RawFinding>, threshold: f32) -> Vec<RawFinding> {
        findings
            .into_iter()
            .filter(|f| f.relevance >= threshold)
            .collect()
    }

    fn synthesize_findings(&self, raw_findings: Vec<RawFinding>) -> Vec<Finding> {
        raw_findings
            .into_iter()
            .map(|rf| Finding {
                title: "Research Finding".to_string(),
                description: rf.content,
                relevance: rf.relevance,
                confidence: rf.relevance,
                sources: vec![rf.source],
                tags: vec!["research".to_string()],
            })
            .collect()
    }

    fn generate_summary(&self, findings: &[Finding]) -> String {
        format!(
            "Research completed with {} key findings. Average confidence: {:.2}",
            findings.len(),
            findings.iter().map(|f| f.confidence).sum::<f32>() / findings.len() as f32
        )
    }

    fn calculate_confidence(&self, findings: &[Finding]) -> f32 {
        if findings.is_empty() {
            return 0.0;
        }
        findings.iter().map(|f| f.confidence).sum::<f32>() / findings.len() as f32
    }

    fn generate_recommendations(&self, findings: &[Finding]) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !findings.is_empty() {
            recommendations.push("Review findings and validate with domain experts".to_string());
            recommendations.push("Consider conducting follow-up research on related topics".to_string());
        }

        recommendations
    }

    fn identify_related_topics(&self, _query: &ResearchQuery, _findings: &[Finding]) -> Vec<String> {
        vec!["Related Topic 1".to_string(), "Related Topic 2".to_string()]
    }

    fn get_consulted_sources(&self) -> Vec<Source> {
        vec![Source {
            title: "Knowledge Base".to_string(),
            url: "internal://kb".to_string(),
            source_type: InformationSource::KnowledgeBase,
            quality_score: 0.9,
            last_accessed: Utc::now(),
        }]
    }

    fn collect_trend_data(&self, _topic: &str, _time_range: &TimeRange) -> Vec<TrendDataPoint> {
        vec![
            TrendDataPoint {
                timestamp: Utc::now(),
                value: 0.5,
                label: "Data point".to_string(),
            },
        ]
    }

    fn determine_trend_direction(&self, data_points: &[TrendDataPoint]) -> TrendDirection {
        if data_points.len() < 2 {
            return TrendDirection::Stable;
        }

        let first_value = data_points[0].value;
        let last_value = data_points[data_points.len() - 1].value;

        if last_value > first_value * 1.1 {
            TrendDirection::Rising
        } else if last_value < first_value * 0.9 {
            TrendDirection::Falling
        } else {
            TrendDirection::Stable
        }
    }

    fn calculate_trend_strength(&self, data_points: &[TrendDataPoint]) -> f32 {
        if data_points.len() < 2 {
            return 0.0;
        }

        let values: Vec<f32> = data_points.iter().map(|dp| dp.value).collect();
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;

        variance.sqrt() / mean.max(0.001)
    }

    fn generate_trend_predictions(
        &self,
        _data_points: &[TrendDataPoint],
        direction: &TrendDirection,
    ) -> Vec<String> {
        match direction {
            TrendDirection::Rising => vec!["Expect continued growth".to_string()],
            TrendDirection::Falling => vec!["Anticipate decline".to_string()],
            TrendDirection::Stable => vec!["Stable outlook".to_string()],
            TrendDirection::Volatile => vec!["High uncertainty".to_string()],
        }
    }

    fn research_technologies(&self, _technologies: &[String]) -> Result<Vec<TechnologyData>> {
        Ok(vec![])
    }

    fn compare_dimensions(
        &self,
        _tech_data: &[TechnologyData],
        dimensions: &[String],
    ) -> Vec<ComparisonDimension> {
        dimensions
            .iter()
            .map(|dim| ComparisonDimension {
                name: dim.clone(),
                scores: HashMap::new(),
                notes: "Comparison analysis".to_string(),
            })
            .collect()
    }

    fn generate_technology_recommendation(&self, _dimensions: &[ComparisonDimension]) -> String {
        "Based on the analysis, consider the trade-offs carefully".to_string()
    }

    fn verify_claim(&self, claim: &str) -> Result<FactCheckResult> {
        Ok(FactCheckResult {
            claim: claim.to_string(),
            verdict: FactCheckVerdict::Unverified,
            confidence: 0.5,
            evidence: vec![],
            notes: "Fact checking requires external sources".to_string(),
        })
    }
}

impl Agent for ResearcherAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Researcher
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}

// Supporting types

#[derive(Debug, Clone)]
struct RawFinding {
    content: String,
    source: String,
    relevance: f32,
}

#[derive(Debug, Clone)]
struct TechnologyData {
    name: String,
    attributes: HashMap<String, String>,
}

/// Fact check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactCheckResult {
    pub claim: String,
    pub verdict: FactCheckVerdict,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub notes: String,
}

/// Verdict of fact checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FactCheckVerdict {
    True,
    False,
    PartiallyTrue,
    Misleading,
    Unverified,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_researcher_agent_creation() {
        let agent = ResearcherAgent::new("TestResearcher".to_string());
        assert_eq!(agent.name(), "TestResearcher");
        assert_eq!(agent.agent_type(), AgentType::Researcher);
        assert!(agent.capabilities().contains(&Capability::InformationRetrieval));
        assert!(agent.capabilities().contains(&Capability::TechnologyResearch));
    }

    #[test]
    fn test_search_strategies() {
        let agent = ResearcherAgent::new("TestResearcher".to_string());
        let strategies = agent.get_search_strategies();
        assert!(!strategies.is_empty());
    }

    #[test]
    fn test_research_query() {
        let agent = ResearcherAgent::new("TestResearcher".to_string());
        let query = ResearchQuery {
            query: "Best practices for Rust async programming".to_string(),
            query_type: QueryType::BestPractices,
            scope: ResearchScope::Public,
            max_results: 10,
            time_range: None,
            quality_threshold: 0.7,
        };

        let result = agent.research(query);
        assert!(result.is_ok());

        let report = result.unwrap();
        assert!(!report.summary.is_empty());
        assert!(report.confidence >= 0.0 && report.confidence <= 1.0);
    }

    #[test]
    fn test_trend_analysis() {
        let agent = ResearcherAgent::new("TestResearcher".to_string());
        let time_range = TimeRange {
            start: Utc::now() - chrono::Duration::days(30),
            end: Utc::now(),
        };

        let result = agent.analyze_trends("Rust adoption".to_string(), time_range);
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert!(!analysis.summary.is_empty());
        assert!(analysis.strength >= 0.0);
    }

    #[test]
    fn test_technology_comparison() {
        let agent = ResearcherAgent::new("TestResearcher".to_string());
        let technologies = vec!["Rust".to_string(), "Go".to_string(), "C++".to_string()];
        let dimensions = vec!["Performance".to_string(), "Safety".to_string()];

        let result = agent.compare_technologies(technologies.clone(), dimensions.clone());
        assert!(result.is_ok());

        let comparison = result.unwrap();
        assert_eq!(comparison.technologies, technologies);
        assert_eq!(comparison.dimensions.len(), dimensions.len());
    }

    #[test]
    fn test_fact_checking() {
        let agent = ResearcherAgent::new("TestResearcher".to_string());
        let claims = vec!["Rust is memory safe".to_string()];

        let result = agent.check_facts(claims);
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_custom_strategies() {
        let custom_strategies = vec![SearchStrategy::DomainExpert];
        let custom_sources = vec![InformationSource::AcademicPapers];

        let agent = ResearcherAgent::with_strategies(
            "CustomResearcher".to_string(),
            custom_strategies.clone(),
            custom_sources.clone(),
        );

        assert_eq!(agent.get_search_strategies().len(), custom_strategies.len());
        assert_eq!(agent.get_information_sources().len(), custom_sources.len());
    }
}
