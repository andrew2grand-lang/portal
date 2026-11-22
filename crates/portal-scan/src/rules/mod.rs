//! Rules: detectors that report hits; the catalog supplies the words.

pub mod catalog;
pub mod files;
pub mod js;
pub mod project;
pub mod python;
pub mod scripts;

use std::path::Path;

use crate::report::{Finding, Severity, State, TargetOs};
use crate::snapshot::Snapshot;
use crate::walk::Inventory;

/// Places listed per rule; the rest are counted, not listed.
pub const MAX_PER_RULE: usize = 50;

/// One thing a detector found.
#[derive(Clone, Debug)]
pub struct Hit {
    pub rule: &'static str,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub evidence: Option<String>,
    pub state: Option<State>,
    pub severity: Option<Severity>,
    /// Narrows which directions this hit affects (e.g. a Windows path only matters on a Mac).
    pub targets: Option<Vec<TargetOs>>,
    /// Fills `{name}` placeholders in the rule's title, why and fix.
    pub vars: Vec<(&'static str, String)>,
}

impl Hit {
    pub fn new(rule: &'static str) -> Self {
        Hit { rule, file: None, line: None, evidence: None, state: None, severity: None, targets: None, vars: vec![] }
    }

    pub fn at(mut self, file: &str, line: u32) -> Self {
        self.file = Some(file.to_owned());
        self.line = Some(line);
        self
    }

    pub fn file(mut self, file: &str) -> Self {
        self.file = Some(file.to_owned());
        self
    }

    pub fn evidence(mut self, e: impl Into<String>) -> Self {
        self.evidence = Some(crate::parse::shorten(&e.into(), 160));
        self
    }

    pub fn state(mut self, s: State) -> Self {
        self.state = Some(s);
        self
    }

    pub fn severity(mut self, s: Severity) -> Self {
        self.severity = Some(s);
        self
    }

    pub fn only(mut self, t: TargetOs) -> Self {
        self.targets = Some(vec![t]);
        self
    }

    pub fn var(mut self, k: &'static str, v: impl Into<String>) -> Self {
        self.vars.push((k, v.into()));
        self
    }
}

/// What every rule may look at.
pub struct Ctx<'a> {
    pub root: &'a Path,
    pub target: TargetOs,
    pub inv: &'a Inventory,
    pub snap: &'a Snapshot,
}

fn fill(template: &str, hit: &Hit, target: TargetOs) -> String {
    let mut out = template.replace("{target}", target.name());
    for (k, v) in &hit.vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// A hit as a finding for this target; None if it doesn't affect it.
pub fn finding(hit: &Hit, target: TargetOs) -> Option<Finding> {
    let meta = catalog::meta(hit.rule);
    let targets = hit.targets.clone().unwrap_or_else(|| meta.targets.to_vec());
    if !targets.contains(&target) {
        return None;
    }
    let id = format!(
        "{}:{}:{}",
        hit.rule,
        hit.file.as_deref().unwrap_or("-"),
        hit.line.map(|l| l.to_string()).unwrap_or_else(|| "-".into())
    );
    Some(Finding {
        id,
        rule: hit.rule.to_owned(),
        title: fill(meta.title, hit, target),
        category: meta.category,
        state: hit.state.unwrap_or(meta.state),
        severity: hit.severity.unwrap_or(meta.severity),
        file: hit.file.clone(),
        line: hit.line,
        evidence: hit.evidence.clone(),
        why: fill(meta.why, hit, target),
        fix_pattern: fill(meta.fix, hit, target),
        auto_fixable: meta.auto_fixable,
        targets,
    })
}
