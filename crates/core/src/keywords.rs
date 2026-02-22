use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentMode {
    Execute,
    Plan,
    PlanAndExecute,
    Debug,
}

impl Default for AgentMode {
    fn default() -> Self {
        Self::Execute
    }
}

impl std::fmt::Display for AgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentMode::Execute => write!(f, "execute"),
            AgentMode::Plan => write!(f, "plan"),
            AgentMode::PlanAndExecute => write!(f, "plan+execute"),
            AgentMode::Debug => write!(f, "debug"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TurnFlags {
    pub plan: bool,
    pub persist: bool,
    pub parallel: bool,
    pub tdd: bool,
    pub eco: bool,
    pub review: bool,
    pub think: bool,
    pub deep: bool,
    pub ultra: bool,
    pub debug: bool,
}

impl TurnFlags {
    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut labels = vec![];
        if self.plan { labels.push("plan"); }
        if self.persist { labels.push("persist"); }
        if self.parallel { labels.push("parallel"); }
        if self.tdd { labels.push("tdd"); }
        if self.eco { labels.push("eco"); }
        if self.review { labels.push("review"); }
        if self.think { labels.push("think"); }
        if self.deep { labels.push("deep"); }
        if self.ultra { labels.push("ultra"); }
        if self.debug { labels.push("debug"); }
        labels
    }

    pub fn any(&self) -> bool {
        self.plan || self.persist || self.parallel || self.tdd || self.eco
            || self.review || self.think || self.deep || self.ultra || self.debug
    }

    pub fn inferred_mode(&self) -> AgentMode {
        if self.debug {
            AgentMode::Debug
        } else if self.plan {
            AgentMode::Plan
        } else {
            AgentMode::Execute
        }
    }

    pub fn thinking_level(&self) -> u32 {
        if self.ultra { 3 }
        else if self.deep { 2 }
        else if self.think { 1 }
        else { 0 }
    }
}

pub fn detect_keywords(prompt: &str) -> (TurnFlags, String) {
    let mut flags = TurnFlags::default();
    let mut cleaned = prompt.to_string();

    type KeywordSetter = (&'static str, fn(&mut TurnFlags));
    let keywords: &[KeywordSetter] = &[
        ("plan:", |f| f.plan = true),
        ("persist:", |f| f.persist = true),
        ("parallel:", |f| f.parallel = true),
        ("tdd:", |f| f.tdd = true),
        ("eco:", |f| f.eco = true),
        ("review:", |f| f.review = true),
        ("think:", |f| f.think = true),
        ("deep:", |f| f.deep = true),
        ("ultra:", |f| f.ultra = true),
        ("debug:", |f| f.debug = true),
    ];

    let mut found: HashSet<&str> = HashSet::new();
    let lower = prompt.to_lowercase();

    for (kw, setter) in keywords {
        if lower.starts_with(kw) || lower.contains(&format!(" {kw}")) {
            setter(&mut flags);
            found.insert(kw);
        }
    }

    for kw in &found {
        if let Some(pos) = cleaned.to_lowercase().find(kw) {
            cleaned.replace_range(pos..pos + kw.len(), "");
        }
    }

    let cleaned = cleaned.trim().to_string();
    (flags, cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_plan() {
        let (flags, cleaned) = detect_keywords("plan: refactor the auth module");
        assert!(flags.plan);
        assert!(!flags.eco);
        assert_eq!(cleaned, "refactor the auth module");
    }

    #[test]
    fn test_detect_eco() {
        let (flags, cleaned) = detect_keywords("eco: fix this typo");
        assert!(flags.eco);
        assert_eq!(cleaned, "fix this typo");
    }

    #[test]
    fn test_detect_multiple() {
        let (flags, _) = detect_keywords("persist: tdd: build a new feature");
        assert!(flags.persist);
        assert!(flags.tdd);
    }

    #[test]
    fn test_no_keywords() {
        let (flags, cleaned) = detect_keywords("just a normal prompt");
        assert!(!flags.any());
        assert_eq!(cleaned, "just a normal prompt");
    }
}
