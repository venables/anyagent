//! Authoritative run metadata -- the trusted, structured record of what
//! actually ran, written to a side channel (`--meta-file`) distinct from the
//! agent's answer on stdout.
//!
//! The point of this envelope is honesty: `model_resolved` is the launcher's
//! truth (read from the transcript), not the agent's self-report, and it is
//! `"unknown"` when the harness genuinely never exposed the build -- never an
//! echo of the request pretending to be the resolved value.

use crate::args::Options;
use crate::transcript::Summary;

/// The terminal status of a run, mapped to a stable exit code and label. The
/// codes are a real API orchestrators branch on, so they are fixed:
///
/// `0` ok · `10` agent-error · `20` timeout · `30` harness-not-found ·
/// `31` invalid-model · `32` enforcement-unsupported · `130` interrupted ·
/// `2` internal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Ok,
    AgentError,
    Timeout,
    HarnessNotFound,
    // Constructed once model validation (Phase 6) and enforcement (Phase 4)
    // land; defined now so the exit-code taxonomy is stable from the start.
    #[allow(dead_code)]
    InvalidModel,
    #[allow(dead_code)]
    EnforcementUnsupported,
    Interrupted,
    Internal,
}

impl ExitStatus {
    pub fn code(self) -> u8 {
        match self {
            Self::Ok => 0,
            Self::AgentError => 10,
            Self::Timeout => 20,
            Self::HarnessNotFound => 30,
            Self::InvalidModel => 31,
            Self::EnforcementUnsupported => 32,
            Self::Interrupted => 130,
            Self::Internal => 2,
        }
    }

    /// Stable machine-readable label for the metadata envelope.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::AgentError => "agent-error",
            Self::Timeout => "timeout",
            Self::HarnessNotFound => "harness-not-found",
            Self::InvalidModel => "invalid-model",
            Self::EnforcementUnsupported => "enforcement-unsupported",
            Self::Interrupted => "interrupted",
            Self::Internal => "internal",
        }
    }
}

/// The authoritative metadata envelope.
pub struct Metadata {
    pub harness: String,
    /// Best-effort harness version; `None` (serialized `null`) when unprobed.
    pub harness_version: Option<String>,
    /// The model the caller asked for; `"default"` when none was specified.
    pub model_requested: String,
    /// The model the harness actually ran, per the transcript; `"unknown"`
    /// when the harness never exposed it. Never an echo of the request.
    pub model_resolved: String,
    pub duration_ms: u64,
    pub exit_status: ExitStatus,
    pub session_id: String,
    pub num_turns: u32,
    pub total_cost_usd: f64,
    pub usage: crate::transcript::Usage,
}

impl Metadata {
    /// Build the envelope from the request and (when available) the run's
    /// summary. `summary` is `None` when the run failed before producing one.
    pub fn build(
        opts: &Options,
        summary: Option<&Summary>,
        duration_ms: u64,
        exit_status: ExitStatus,
    ) -> Self {
        let model_requested = opts.model.clone().unwrap_or_else(|| "default".to_string());
        let model_resolved = summary
            .map(|s| s.model.as_str())
            .filter(|m| !m.is_empty())
            .unwrap_or("unknown")
            .to_string();
        Self {
            harness: opts.harness.name().to_string(),
            harness_version: None,
            model_requested,
            model_resolved,
            duration_ms,
            exit_status,
            session_id: summary.map(|s| s.session_id.clone()).unwrap_or_default(),
            num_turns: summary.map(|s| s.num_turns).unwrap_or(0),
            total_cost_usd: summary.map(|s| s.total_cost_usd).unwrap_or(0.0),
            usage: summary.map(|s| s.usage.clone()).unwrap_or_default(),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "harness": self.harness,
            "harness_version": self.harness_version,
            "model_requested": self.model_requested,
            "model_resolved": self.model_resolved,
            "duration_ms": self.duration_ms,
            "exit_status": self.exit_status.label(),
            "session_id": self.session_id,
            "num_turns": self.num_turns,
            "total_cost_usd": self.total_cost_usd,
            "usage": {
                "input_tokens": self.usage.input_tokens,
                "output_tokens": self.usage.output_tokens,
                "cache_read_input_tokens": self.usage.cache_read_input_tokens,
                "cache_creation_input_tokens": self.usage.cache_creation_input_tokens,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::{Summary, Usage};

    fn summary() -> Summary {
        Summary {
            final_text: "hi".into(),
            session_id: "sid".into(),
            model: "claude-opus-4-8".into(),
            is_error: false,
            num_turns: 2,
            total_cost_usd: 0.01,
            duration_api_ms: 0,
            usage: Usage {
                input_tokens: 12,
                output_tokens: 8,
                ..Default::default()
            },
            jsonl_replay: String::new(),
        }
    }

    #[test]
    fn exit_codes_are_stable() {
        assert_eq!(ExitStatus::Ok.code(), 0);
        assert_eq!(ExitStatus::AgentError.code(), 10);
        assert_eq!(ExitStatus::Timeout.code(), 20);
        assert_eq!(ExitStatus::HarnessNotFound.code(), 30);
        assert_eq!(ExitStatus::InvalidModel.code(), 31);
        assert_eq!(ExitStatus::EnforcementUnsupported.code(), 32);
        assert_eq!(ExitStatus::Interrupted.code(), 130);
        assert_eq!(ExitStatus::Internal.code(), 2);
    }

    #[test]
    fn resolved_model_comes_from_summary() {
        let opts = Options {
            model: Some("opus".into()),
            ..Options::default()
        };
        let m = Metadata::build(&opts, Some(&summary()), 100, ExitStatus::Ok);
        assert_eq!(m.model_requested, "opus");
        assert_eq!(m.model_resolved, "claude-opus-4-8");
        assert_eq!(m.to_json()["exit_status"], "ok");
        assert_eq!(m.to_json()["usage"]["input_tokens"], 12);
    }

    #[test]
    fn requested_default_when_unspecified() {
        let m = Metadata::build(&Options::default(), Some(&summary()), 1, ExitStatus::Ok);
        assert_eq!(m.model_requested, "default");
    }

    #[test]
    fn resolved_unknown_without_summary_or_model() {
        // No summary at all (failed run).
        let m = Metadata::build(&Options::default(), None, 1, ExitStatus::Timeout);
        assert_eq!(m.model_resolved, "unknown");

        // Summary present but transcript never exposed the model.
        let mut s = summary();
        s.model = String::new();
        let m = Metadata::build(&Options::default(), Some(&s), 1, ExitStatus::Ok);
        assert_eq!(m.model_resolved, "unknown");
    }
}
