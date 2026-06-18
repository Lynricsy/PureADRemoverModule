use crate::manifest::Category;

mod executable;
mod forbidden;
mod useful;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Decision {
    Accepted,
    Rejected,
    Ignored,
}

#[derive(Debug)]
pub struct Classification {
    pub decision: Decision,
    pub categories: Vec<Category>,
    pub signals: Vec<String>,
    pub reason: String,
}

#[derive(Default)]
pub struct Findings {
    pub categories: Vec<Category>,
    pub signals: Vec<String>,
    reasons: Vec<&'static str>,
}

pub fn classify(display_path: &str, content: &[u8]) -> Classification {
    let path = display_path.to_lowercase();
    let text = String::from_utf8_lossy(content).to_lowercase();

    let mut forbidden = Findings::default();
    forbidden::classify(&path, &text, &mut forbidden);
    if !forbidden.categories.is_empty() {
        return forbidden.into_classification(Decision::Rejected, "forbidden upstream scope");
    }

    let mut useful = Findings::default();
    useful::classify(&path, &text, &mut useful);
    if !useful.categories.is_empty() {
        return useful.into_classification(Decision::Accepted, "local non-domain ad material");
    }

    Classification {
        decision: Decision::Ignored,
        categories: Vec::new(),
        signals: Vec::new(),
        reason: "no local non-domain ad signal found".to_owned(),
    }
}

pub fn is_executable_upstream_code(display_path: &str, content: &[u8]) -> bool {
    executable::is_executable(display_path, content)
}

impl Findings {
    pub fn add(&mut self, category: Category, signal: &'static str, reason: &'static str) {
        if !self.categories.contains(&category) {
            self.categories.push(category);
        }
        if !self.signals.iter().any(|value| value == signal) {
            self.signals.push(signal.to_owned());
        }
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }
    }

    fn into_classification(mut self, decision: Decision, fallback: &'static str) -> Classification {
        self.categories.sort_unstable();
        self.signals.sort();
        Classification {
            decision,
            categories: self.categories,
            signals: self.signals,
            reason: if self.reasons.is_empty() {
                fallback.to_owned()
            } else {
                self.reasons.join("; ")
            },
        }
    }
}

pub fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}
