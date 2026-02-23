use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeConfig {
    pub candidates: usize,
    pub criteria: Vec<JudgeCriterion>,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            candidates: 3,
            criteria: vec![
                JudgeCriterion::TestPassRate,
                JudgeCriterion::DiffSize,
                JudgeCriterion::LintErrors,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JudgeCriterion {
    TestPassRate,
    DiffSize,
    LintErrors,
    TypeErrors,
    ComplexityDelta,
}

#[derive(Debug, Clone)]
pub struct CandidateResult {
    pub id: usize,
    pub worktree_name: String,
    pub branch: String,
    pub scores: Vec<(JudgeCriterion, f64)>,
    pub total_score: f64,
    pub diff_summary: String,
    pub test_output: String,
    pub success: bool,
}

pub struct JudgeSession {
    pub config: JudgeConfig,
    pub prompt: String,
    pub results: Vec<CandidateResult>,
}

impl JudgeSession {
    pub fn new(prompt: &str, config: JudgeConfig) -> Self {
        Self {
            config,
            prompt: prompt.to_string(),
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: CandidateResult) {
        self.results.push(result);
    }

    pub fn rank(&mut self) -> Vec<&CandidateResult> {
        self.results.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.results.iter().collect()
    }

    pub fn best(&self) -> Option<&CandidateResult> {
        self.results.iter().max_by(|a, b| {
            a.total_score
                .partial_cmp(&b.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn format_comparison(&self) -> String {
        if self.results.is_empty() {
            return "No candidates to compare.".to_string();
        }

        let mut lines = vec![format!(
            "Judging {} candidates for: {}",
            self.results.len(),
            self.prompt
        )];
        lines.push(String::new());

        let mut sorted = self.results.clone();
        sorted.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (rank, candidate) in sorted.iter().enumerate() {
            let status = if candidate.success { "PASS" } else { "FAIL" };
            lines.push(format!(
                "#{} [{}] Candidate {} (branch: {}) - Score: {:.2}",
                rank + 1,
                status,
                candidate.id,
                candidate.branch,
                candidate.total_score
            ));
            for (criterion, score) in &candidate.scores {
                lines.push(format!("    {:?}: {:.2}", criterion, score));
            }
            if !candidate.diff_summary.is_empty() {
                lines.push(format!("    Diff: {}", candidate.diff_summary));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

pub fn score_test_output(output: &str) -> f64 {
    let lower = output.to_lowercase();
    if lower.contains("test result: ok")
        || lower.contains("tests passed")
        || lower.contains("0 failed")
    {
        1.0
    } else if lower.contains("failed") || lower.contains("error") {
        let fail_count = lower.matches("failed").count() + lower.matches("error").count();
        let total_lines = output.lines().count().max(1) as f64;
        (1.0 - (fail_count as f64 / total_lines)).max(0.0)
    } else {
        0.5
    }
}

pub fn score_diff_size(diff: &str) -> f64 {
    let lines: usize = diff.lines().count();
    if lines == 0 {
        0.0
    } else if lines < 50 {
        1.0
    } else if lines < 200 {
        0.8
    } else if lines < 500 {
        0.5
    } else {
        0.3
    }
}

pub fn score_lint_output(output: &str) -> f64 {
    let lower = output.to_lowercase();
    let error_count = lower.matches("error").count();
    let warning_count = lower.matches("warning").count();
    if error_count == 0 && warning_count == 0 {
        1.0
    } else if error_count == 0 {
        0.8
    } else {
        (1.0 - (error_count as f64 * 0.2)).max(0.0)
    }
}

pub fn compute_total_score(scores: &[(JudgeCriterion, f64)]) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    let weights: Vec<f64> = scores
        .iter()
        .map(|(c, _)| match c {
            JudgeCriterion::TestPassRate => 3.0,
            JudgeCriterion::LintErrors => 2.0,
            JudgeCriterion::TypeErrors => 2.0,
            JudgeCriterion::DiffSize => 1.0,
            JudgeCriterion::ComplexityDelta => 1.0,
        })
        .collect();
    let total_weight: f64 = weights.iter().sum();
    let weighted_sum: f64 = scores
        .iter()
        .zip(weights.iter())
        .map(|((_, score), weight)| score * weight)
        .sum();
    weighted_sum / total_weight
}
