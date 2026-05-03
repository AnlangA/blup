use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub source_chunk_id: Uuid,
    pub target_message_id: Uuid,
    pub relevance_score: f32,
    pub usage_type: CitationUsageType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CitationUsageType {
    DirectQuote,
    Paraphrase,
    Background,
    Example,
}

pub struct CitationTracker {
    /// Maps message_id -> Vec<Citation>
    citations: HashMap<Uuid, Vec<Citation>>,
}

impl CitationTracker {
    pub fn new() -> Self {
        Self {
            citations: HashMap::new(),
        }
    }

    /// Add a citation linking a source chunk to a message
    pub fn add_citation(
        &mut self,
        source_chunk_id: Uuid,
        target_message_id: Uuid,
        relevance_score: f32,
        usage_type: CitationUsageType,
    ) {
        let citation = Citation {
            source_chunk_id,
            target_message_id,
            relevance_score,
            usage_type,
        };

        self.citations
            .entry(target_message_id)
            .or_default()
            .push(citation);
    }

    /// Get all citations for a message
    pub fn get_citations(&self, message_id: &Uuid) -> Option<&Vec<Citation>> {
        self.citations.get(message_id)
    }

    /// Get all citations
    pub fn get_all_citations(&self) -> &HashMap<Uuid, Vec<Citation>> {
        &self.citations
    }

    /// Get count of citations for a message
    pub fn citation_count(&self, message_id: &Uuid) -> usize {
        self.citations.get(message_id).map(|c| c.len()).unwrap_or(0)
    }

    /// Check if a message has any citations
    pub fn has_citations(&self, message_id: &Uuid) -> bool {
        self.citation_count(message_id) > 0
    }

    /// Get all unique source chunk IDs cited by a message
    pub fn get_source_chunks(&self, message_id: &Uuid) -> Vec<Uuid> {
        self.citations
            .get(message_id)
            .map(|citations| citations.iter().map(|c| c.source_chunk_id).collect())
            .unwrap_or_default()
    }
}

impl Default for CitationTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_citation() {
        let mut tracker = CitationTracker::new();
        let chunk_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        tracker.add_citation(chunk_id, message_id, 0.9, CitationUsageType::DirectQuote);

        let citations = tracker.get_citations(&message_id).unwrap();
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].source_chunk_id, chunk_id);
    }

    #[test]
    fn test_multiple_citations() {
        let mut tracker = CitationTracker::new();
        let message_id = Uuid::new_v4();

        tracker.add_citation(
            Uuid::new_v4(),
            message_id,
            0.8,
            CitationUsageType::Paraphrase,
        );
        tracker.add_citation(
            Uuid::new_v4(),
            message_id,
            0.6,
            CitationUsageType::Background,
        );

        assert_eq!(tracker.citation_count(&message_id), 2);
        assert!(tracker.has_citations(&message_id));
    }

    #[test]
    fn test_no_citations() {
        let tracker = CitationTracker::new();
        let message_id = Uuid::new_v4();

        assert_eq!(tracker.citation_count(&message_id), 0);
        assert!(!tracker.has_citations(&message_id));
        assert!(tracker.get_source_chunks(&message_id).is_empty());
    }
}
