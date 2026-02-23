use std::path::Path;

use crate::verify::{self, VerifyReport};

pub struct PersistenceConfig {
    pub max_iterations: u32,
    pub project_root: std::path::PathBuf,
    pub cwd: std::path::PathBuf,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            project_root: std::path::PathBuf::from("."),
            cwd: std::path::PathBuf::from("."),
        }
    }
}

pub async fn run_qa_cycle(project_root: &Path, cwd: &Path, max_cycles: u32) -> Vec<VerifyReport> {
    let checks = verify::detect_checks(project_root);
    if checks.is_empty() {
        return vec![];
    }

    let mut reports = vec![];
    for _cycle in 0..max_cycles {
        let report = verify::run_all_checks(&checks, cwd).await;
        let passed = report.all_passed();
        reports.push(report);
        if passed {
            break;
        }
    }
    reports
}

pub fn qa_summary(reports: &[VerifyReport]) -> String {
    if reports.is_empty() {
        return "No verification checks detected.".to_string();
    }

    let mut lines = vec![format!("QA ran {} cycle(s):", reports.len())];
    for (i, report) in reports.iter().enumerate() {
        let status = if report.all_passed() { "PASS" } else { "FAIL" };
        let check_count = report.checks.len();
        let pass_count = report.checks.iter().filter(|c| c.passed()).count();
        lines.push(format!(
            "  Cycle {}: [{status}] {pass_count}/{check_count} checks passed",
            i + 1,
        ));
    }

    if let Some(last) = reports.last() {
        if last.all_passed() {
            lines.push("All checks passed.".to_string());
        } else {
            lines.push(String::new());
            lines.push("Last cycle failures:".to_string());
            lines.push(last.summary());
        }
    }

    lines.join("\n")
}
