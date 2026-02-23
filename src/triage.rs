//------------------------------------------------------------------------------------------------------------
// file: triage.rs
// purpose: Prompt signal scoring and tier routing for API proxy mode
//
// v0.6.0 - Routes prompts to cache, local ollama, or upstream API
//------------------------------------------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// Which tier handles this prompt
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Tier {
    Cache,
    Local,
    Upstream,
}

/// Result of evaluating a prompt
#[derive(Debug, Serialize)]
pub struct RouteDecision {
    pub tier: Tier,
    pub reason: String,
    pub confidence: f64,
}

/// Upstream signal patterns — complex reasoning, architecture, domain-specific
const UPSTREAM_SIGNALS: &[&str] = &[
    "analyze", "architect", "design", "implement", "refactor", "synthesize",
    "phext", "choir", "tessera", "consciousness", "exocortex", "mirrorborn",
    "essay", "report", "specification", "codebase", "migration",
    "prove", "derive", "theorem", "axiom",
];

/// Local signal patterns — translation, formatting, quick tasks
const LOCAL_SIGNALS: &[&str] = &[
    "summarize", "translate", "classify", "extract", "format", "convert",
    "define", "explain briefly", "fix typo", "fix grammar", "rewrite", "rephrase",
    "bash", "shell", "snippet", "json", "yaml", "regex", "grep",
];

/// Score a prompt and decide routing tier
pub fn evaluate(prompt: &str, upstream_threshold: usize) -> RouteDecision {
    let lower = prompt.to_lowercase();
    let word_count = prompt.split_whitespace().count();

    let mut upstream_score: usize = 0;
    let mut local_score: usize = 0;

    for signal in UPSTREAM_SIGNALS {
        if lower.contains(signal) {
            upstream_score += 1;
        }
    }

    for signal in LOCAL_SIGNALS {
        if lower.contains(signal) {
            local_score += 1;
        }
    }

    // Decision logic
    if upstream_score >= upstream_threshold {
        RouteDecision {
            tier: Tier::Upstream,
            reason: format!("{} upstream signal(s), {} words", upstream_score, word_count),
            confidence: 0.9,
        }
    } else if local_score > 0 || word_count < 50 {
        RouteDecision {
            tier: Tier::Local,
            reason: format!("{} local signal(s), {} words", local_score, word_count),
            confidence: if local_score > 0 { 0.8 } else { 0.6 },
        }
    } else {
        // Long prompt with no signals → upstream (conservative)
        RouteDecision {
            tier: Tier::Upstream,
            reason: format!("no signals, {} words (conservative)", word_count),
            confidence: 0.5,
        }
    }
}

/// Feedback loop — tracks local model quality
pub struct FeedbackLoop {
    window: Vec<bool>,
    max_window: usize,
}

impl FeedbackLoop {
    pub fn new(max_window: usize) -> Self {
        FeedbackLoop {
            window: Vec::new(),
            max_window,
        }
    }

    pub fn record(&mut self, success: bool) {
        if self.window.len() >= self.max_window {
            self.window.remove(0);
        }
        self.window.push(success);
    }

    /// Returns true if failure rate exceeds threshold (should escalate)
    pub fn should_escalate(&self, threshold: f64) -> bool {
        if self.window.is_empty() {
            return false;
        }
        let failures = self.window.iter().filter(|&&s| !s).count();
        (failures as f64 / self.window.len() as f64) > threshold
    }

    pub fn failure_rate(&self) -> f64 {
        if self.window.is_empty() {
            return 0.0;
        }
        let failures = self.window.iter().filter(|&&s| !s).count();
        failures as f64 / self.window.len() as f64
    }
}

#[cfg(test)]
mod triage_tests {
    use super::*;

    #[test]
    fn test_upstream_analyze() {
        let d = evaluate("analyze this codebase for performance issues", 1);
        assert_eq!(d.tier, Tier::Upstream);
    }

    #[test]
    fn test_upstream_design() {
        let d = evaluate("design a new architecture for the exocortex", 1);
        assert_eq!(d.tier, Tier::Upstream);
    }

    #[test]
    fn test_local_summarize() {
        let d = evaluate("summarize this document", 1);
        assert_eq!(d.tier, Tier::Local);
    }

    #[test]
    fn test_local_convert() {
        let d = evaluate("convert this json to yaml", 1);
        assert_eq!(d.tier, Tier::Local);
    }

    #[test]
    fn test_both_signals_upstream_wins() {
        let d = evaluate("summarize and analyze this phext codebase", 1);
        assert_eq!(d.tier, Tier::Upstream);
    }

    #[test]
    fn test_short_no_signal_local() {
        let d = evaluate("what is 2 + 2", 1);
        assert_eq!(d.tier, Tier::Local);
    }

    #[test]
    fn test_long_no_signal_upstream() {
        let long = "word ".repeat(60);
        let d = evaluate(&long, 1);
        assert_eq!(d.tier, Tier::Upstream);
    }

    #[test]
    fn test_empty_prompt() {
        let d = evaluate("", 1);
        assert_eq!(d.tier, Tier::Local);
    }

    #[test]
    fn test_feedback_empty() {
        let fl = FeedbackLoop::new(100);
        assert!(!fl.should_escalate(0.25));
    }

    #[test]
    fn test_feedback_all_success() {
        let mut fl = FeedbackLoop::new(100);
        for _ in 0..10 { fl.record(true); }
        assert!(!fl.should_escalate(0.25));
        assert!((fl.failure_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_feedback_high_failure() {
        let mut fl = FeedbackLoop::new(100);
        for _ in 0..7 { fl.record(false); }
        for _ in 0..3 { fl.record(true); }
        assert!(fl.should_escalate(0.25));
    }

    #[test]
    fn test_feedback_window_slides() {
        let mut fl = FeedbackLoop::new(4);
        fl.record(false);
        fl.record(false);
        fl.record(false);
        fl.record(true);
        fl.record(true);
        fl.record(true);
        fl.record(true);
        // Window: [false, true, true, true] — 25% failure
        assert!(!fl.should_escalate(0.25));
    }
}
