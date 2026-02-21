use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct TurnFlags {
    pub plan: bool,
    pub persist: bool,
    pub parallel: bool,
    pub tdd: bool,
    pub eco: bool,
    pub review: bool,
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
        labels
    }

    pub fn any(&self) -> bool {
        self.plan || self.persist || self.parallel || self.tdd || self.eco || self.review
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
