//! The report: what the scanner found, versioned. The design lab renders
//! this JSON as-is; `docs/scan-report.schema.json` is generated from these types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Bumped on any change a reader has to know about.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub schema_version: u32,
    pub tool: Tool,
    /// RFC 3339, UTC.
    pub scanned_at: String,
    pub project: ProjectInfo,
    /// The OS the scan ran on.
    pub source: Os,
    /// The OS the project is meant to run on next.
    pub target: TargetOs,
    pub network: Network,
    pub duration_ms: u64,
    pub stats: Stats,
    pub tier: TierInfo,
    /// The answer first, in one sentence ("3 things block this on Windows, 7 need a fix.").
    pub headline: String,
    pub counts: Counts,
    pub ai_usage: AiUsage,
    pub ecosystems: Vec<Ecosystem>,
    pub runtimes: Vec<Runtime>,
    pub packages: Vec<PackageSet>,
    pub run_commands: Vec<RunCommand>,
    pub left_out: LeftOut,
    /// Files that hold secrets. Key names only, never values.
    pub secrets: Vec<SecretFile>,
    /// Databases and other services the project expects to be running.
    pub services: Vec<Service>,
    pub findings: Vec<Finding>,
    /// Markdown for the user's own coding AI, when the app can't go deeper.
    pub handoff: Option<String>,
    /// Plain notes about the scan itself ("Nothing was run.", "Offline: …").
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    /// The folder's name.
    pub name: String,
    /// Absolute path on the scanning machine.
    pub path: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Os {
    Macos,
    Windows,
    Linux,
    Other,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::Macos
        } else if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else {
            Os::Other
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum TargetOs {
    Windows,
    Macos,
}

impl TargetOs {
    /// The other one of the pair, from where the scan runs.
    pub fn other_than_here() -> TargetOs {
        if cfg!(windows) { TargetOs::Macos } else { TargetOs::Windows }
    }

    pub fn word(self) -> &'static str {
        match self {
            TargetOs::Windows => "Windows",
            TargetOs::Macos => "a Mac",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            TargetOs::Windows => "Windows",
            TargetOs::Macos => "macOS",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    /// `--online` was given.
    pub online: bool,
    /// Hosts actually contacted (empty when offline).
    pub hosts: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Files that count as the project (after .gitignore and left-out folders).
    pub files: u64,
    pub bytes: u64,
    /// Source files parsed for code rules.
    pub parsed_files: u64,
    /// Code files skipped: too large, binary or not text.
    pub skipped_files: u64,
    /// Files or folders that couldn't be read.
    pub unreadable: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Static,
    Scripts,
    Apps,
    Systems,
    Native,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TierInfo {
    pub tier: Tier,
    /// One line: "Apps: a FastAPI server plus a local model via Ollama".
    pub reason: String,
}

/// How much a paid conversion would likely lean on the AI. Always an estimate.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AiLevel {
    None,
    Rare,
    Sometimes,
    Often,
    PlanOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiUsage {
    pub level: AiLevel,
    /// Always true: this is an estimate, not a measurement.
    pub estimate: bool,
    pub reason: String,
}

/// Distinct issues (one per rule and state) and how often they occur.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub blocker: u32,
    pub will_need_a_fix: u32,
    pub needs_you: u32,
    pub couldnt_check: u32,
    pub info: u32,
    /// All findings, counting each place separately.
    pub occurrences: u32,
    /// Places not listed because a rule hit its cap.
    pub omitted: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ecosystem {
    /// "python", "node", "shell", "docker", "swift", "dotnet", "go"…
    pub kind: String,
    /// Human name: "Python", "Node.js", "Shell scripts"…
    pub name: String,
    /// Files that showed it, relative.
    pub evidence: Vec<String>,
    /// Source files of this kind in the project.
    pub files: u64,
    /// The app's depth of support: "deep" (rules run) or "summary" (detected, hand-off note).
    pub support: Support,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Support {
    Deep,
    Summary,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
    /// "python" or "node".
    pub kind: String,
    /// What the project asks for (">=3.11", "20"), if it says.
    pub requested: Option<String>,
    /// Where that came from ("pyproject.toml", ".nvmrc").
    pub requested_from: Option<String>,
    /// What `python --version` / `node --version` said on this machine.
    pub installed_here: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageSet {
    /// "python" or "node".
    pub kind: String,
    /// Where they were read from: ".venv", "node_modules", or a manifest.
    pub source: String,
    /// The lockfile, if there is one. None: this snapshot becomes the lock.
    pub lockfile: Option<String>,
    /// Packages declared in manifests.
    pub declared: u32,
    /// Packages installed here (read statically from metadata).
    pub installed: u32,
    /// Up to 200 installed (or declared) packages.
    pub items: Vec<Package>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunCommand {
    /// "pyproject-script", "npm-script", "make", "procfile", "vscode", "main-guard", "readme".
    pub kind: String,
    pub name: String,
    /// The command as written (never run).
    pub command: String,
    pub file: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LeftOut {
    pub bytes: u64,
    pub files: u64,
    pub items: Vec<LeftOutItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LeftOutItem {
    pub path: String,
    /// "pythonEnv", "nodeModules", "cache", "buildOutput", "gitignored", "osJunk".
    pub kind: String,
    pub bytes: u64,
    pub files: u64,
    /// Rebuilt on the other side (environments, caches, build output).
    pub rebuildable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecretFile {
    pub path: String,
    /// "env", "key", "credentials".
    pub kind: String,
    /// Variable names only (for .env files); never values.
    pub keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    /// "postgres", "redis", "mysql", "mongodb", "ollama"…
    pub kind: String,
    pub name: String,
    pub from: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Blocker,
    WillNeedAFix,
    NeedsYou,
    CouldntCheck,
    Info,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    High,
    Medium,
    Low,
}

/// After the MSR 2026 study of cross-OS issues in Python projects (see
/// docs/THIRD_PARTY.md), plus the app's own.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    FilesPaths,
    ProcessesSignals,
    LibraryDependencies,
    ApiAvailability,
    EnvironmentDisplay,
    PermissionsLimits,
    SystemInformation,
    CopiedEnvironments,
    ScriptsLineEndings,
    LocalAiHardware,
    Docker,
    InterpreterOnWindows,
    NativeApps,
    Secrets,
    Services,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Stable: rule, file and line.
    pub id: String,
    pub rule: String,
    /// Short and plain: "Opens files without saying they're UTF-8".
    pub title: String,
    pub category: Category,
    pub state: State,
    pub severity: Severity,
    /// Relative, with forward slashes.
    pub file: Option<String>,
    /// 1-based.
    pub line: Option<u32>,
    /// A short snippet of what was found (never a secret value).
    pub evidence: Option<String>,
    /// One plain sentence.
    pub why: String,
    pub fix_pattern: String,
    /// A paid conversion could apply the fix without asking (for later).
    pub auto_fixable: bool,
    /// Which directions it affects.
    pub targets: Vec<TargetOs>,
}
