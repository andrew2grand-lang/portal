//! `--online`: does each Python dependency ship a ready-made build (wheel)
//! for the target? Reads only PyPI's JSON API; answers are cached for a day.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::report::TargetOs;
use crate::rules::Hit;

/// Where package metadata comes from: PyPI over HTTP, or canned answers in tests.
pub trait PypiSource: Send + Sync {
    /// Ok(Some(json)) found, Ok(None) not on PyPI, Err: couldn't reach it.
    fn project(&self, name: &str, version: Option<&str>) -> Result<Option<serde_json::Value>, String>;
}

pub struct HttpPypi {
    cache: Option<PathBuf>,
}

const CACHE_FOR: Duration = Duration::from_secs(24 * 3600);

impl HttpPypi {
    pub fn new() -> Self {
        HttpPypi { cache: dirs::cache_dir().map(|d| d.join("portal-scan").join("pypi")) }
    }
}

impl Default for HttpPypi {
    fn default() -> Self {
        Self::new()
    }
}

impl PypiSource for HttpPypi {
    fn project(&self, name: &str, version: Option<&str>) -> Result<Option<serde_json::Value>, String> {
        let key = match version {
            Some(v) => format!("{name}=={v}.json"),
            None => format!("{name}.json"),
        };
        if let Some(dir) = &self.cache {
            let path = dir.join(&key);
            let fresh = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .is_some_and(|age| age < CACHE_FOR);
            if fresh && let Ok(text) = std::fs::read_to_string(&path) {
                return Ok(serde_json::from_str(&text).ok());
            }
        }
        let url = match version {
            Some(v) => format!("https://pypi.org/pypi/{name}/{v}/json"),
            None => format!("https://pypi.org/pypi/{name}/json"),
        };
        let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().new_agent();
        match agent
            .get(&url)
            .header("User-Agent", concat!("portal-scan (", env!("SEAMDROP_APP_NAME"), " project scanner)"))
            .call()
        {
            Ok(mut resp) => {
                let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
                let full: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
                // Keep only what's needed: file names and their kind.
                let slim = serde_json::json!({
                    "version": full.pointer("/info/version"),
                    "urls": full.get("urls").and_then(|u| u.as_array()).map(|files| files.iter().map(|f| serde_json::json!({
                        "filename": f.get("filename"),
                        "packagetype": f.get("packagetype"),
                    })).collect::<Vec<_>>()),
                });
                if let Some(dir) = &self.cache {
                    let _ = std::fs::create_dir_all(dir);
                    let _ = std::fs::write(dir.join(&key), slim.to_string());
                }
                Ok(Some(slim))
            }
            Err(ureq::Error::StatusCode(404)) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// A wheel for the target, or a pure-Python one.
    Ready,
    /// Wheels for other systems only: compiled code with nothing for the target.
    OtherSystemsOnly,
    /// Only a source archive: may or may not need a compiler.
    SourceOnly,
    NoFiles,
}

/// Reads one release's file list.
pub fn verdict(json: &serde_json::Value, target: TargetOs) -> Verdict {
    let files: Vec<(String, String)> = json
        .get("urls")
        .and_then(|u| u.as_array())
        .into_iter()
        .flatten()
        .filter_map(|f| {
            Some((f.get("filename")?.as_str()?.to_ascii_lowercase(), f.get("packagetype")?.as_str()?.to_owned()))
        })
        .collect();
    if files.is_empty() {
        return Verdict::NoFiles;
    }
    let wheels: Vec<&String> = files.iter().filter(|(_, k)| k == "bdist_wheel").map(|(n, _)| n).collect();
    let fits = |w: &str| {
        w.ends_with("-any.whl")
            || match target {
                TargetOs::Windows => w.contains("win_amd64"),
                TargetOs::Macos => w.contains("macosx") && (w.contains("arm64") || w.contains("universal2")),
            }
    };
    if wheels.iter().any(|w| fits(w)) {
        Verdict::Ready
    } else if !wheels.is_empty() {
        Verdict::OtherSystemsOnly
    } else {
        Verdict::SourceOnly
    }
}

/// (name, pinned version) for each distinct dependency.
pub fn unique_deps(deps: &[(String, String, String)]) -> Vec<(String, Option<String>)> {
    // Names and versions go into a PyPI URL and a cache file name: only the
    // characters packages actually use (PEP 508 names, PEP 440 versions).
    let plain = |s: &str, extra: &[char]| {
        !s.is_empty()
            && !s.contains("..")
            && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') || extra.contains(&c))
    };
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    for (name, spec, _) in deps {
        if !plain(name, &[]) || out.iter().any(|(n, _)| n == name) {
            continue;
        }
        let pinned = spec.strip_prefix("==").map(str::trim).filter(|v| plain(v, &['+', '!'])).map(str::to_owned);
        out.push((name.clone(), pinned));
    }
    out
}

/// Checks every dependency. Returns the hits and whether PyPI answered at all.
pub fn check(
    deps: &[(String, Option<String>)],
    target: TargetOs,
    source: &dyn PypiSource,
    progress: &dyn Fn(u64, &str),
) -> (Vec<Hit>, bool) {
    let mut hits = Vec::new();
    let mut reached = false;
    let mut failures = 0;
    for (i, (name, version)) in deps.iter().enumerate() {
        progress(i as u64, name);
        match source.project(name, version.as_deref()) {
            Ok(Some(json)) => {
                reached = true;
                let ver = version
                    .clone()
                    .or_else(|| json.get("version").and_then(|v| v.as_str()).map(str::to_owned))
                    .unwrap_or_default();
                let tools = match target {
                    TargetOs::Windows => " (it needs Microsoft C++ Build Tools)",
                    TargetOs::Macos => " (it needs the Xcode Command Line Tools)",
                };
                match verdict(&json, target) {
                    Verdict::Ready => {}
                    Verdict::OtherSystemsOnly => hits.push(
                        Hit::new("no-wheel")
                            .evidence(format!("{name} {ver}"))
                            .var("package", name.clone())
                            .var("version", ver)
                            .var("tools", tools),
                    ),
                    Verdict::SourceOnly | Verdict::NoFiles => hits.push(
                        Hit::new("source-only")
                            .evidence(format!("{name} {ver}"))
                            .var("package", name.clone())
                            .var("version", ver),
                    ),
                }
            }
            Ok(None) => {
                reached = true;
                hits.push(Hit::new("package-not-found").evidence(name.clone()).var("package", name.clone()));
            }
            Err(_) => failures += 1,
        }
    }
    progress(deps.len() as u64, "");
    if failures > 0 {
        hits.push(Hit::new("wheels-unchecked").evidence(format!("{failures} packages couldn't be looked up")));
    }
    (hits, reached)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_names_and_versions_reach_pypi_or_the_cache() {
        let deps = |n: &str, v: &str| vec![(n.to_owned(), format!("=={v}"), "requirements.txt".to_owned())];
        assert_eq!(unique_deps(&deps("requests", "2.32.3")), [("requests".to_owned(), Some("2.32.3".to_owned()))]);
        assert_eq!(unique_deps(&deps("torch", "2.5.1+cpu")), [("torch".to_owned(), Some("2.5.1+cpu".to_owned()))]);
        // A version that would steer the URL or the cache file is treated as unpinned
        for bad in ["2.32.3/json?/../../../x", "1.0/../../peers", "1.0\\..\\x", "1 0"] {
            assert_eq!(unique_deps(&deps("requests", bad)), [("requests".to_owned(), None)], "{bad}");
        }
        // A name that isn't a package name isn't looked up at all
        assert!(unique_deps(&deps("../../etc", "1.0")).is_empty());
        assert!(unique_deps(&deps("a/b", "1.0")).is_empty());
    }

    fn release(files: &[(&str, &str)]) -> serde_json::Value {
        serde_json::json!({ "version": "1.0", "urls": files.iter().map(|(f, k)| serde_json::json!({"filename": f, "packagetype": k})).collect::<Vec<_>>() })
    }

    #[test]
    fn wheels_are_matched_to_the_target() {
        let numpy = release(&[
            ("numpy-2.1.0-cp312-cp312-win_amd64.whl", "bdist_wheel"),
            ("numpy-2.1.0-cp312-cp312-macosx_14_0_arm64.whl", "bdist_wheel"),
            ("numpy-2.1.0.tar.gz", "sdist"),
        ]);
        assert_eq!(verdict(&numpy, TargetOs::Windows), Verdict::Ready);
        assert_eq!(verdict(&numpy, TargetOs::Macos), Verdict::Ready);
        let mac_only = release(&[
            ("pyobjc_core-10-cp312-cp312-macosx_10_9_universal2.whl", "bdist_wheel"),
            ("pyobjc-core-10.tar.gz", "sdist"),
        ]);
        assert_eq!(verdict(&mac_only, TargetOs::Windows), Verdict::OtherSystemsOnly);
        let pure = release(&[("requests-2.32.3-py3-none-any.whl", "bdist_wheel")]);
        assert_eq!(verdict(&pure, TargetOs::Windows), Verdict::Ready);
        let src = release(&[("oldlib-0.1.tar.gz", "sdist")]);
        assert_eq!(verdict(&src, TargetOs::Macos), Verdict::SourceOnly);
        let intel_mac = release(&[("x-1-cp312-cp312-macosx_10_9_x86_64.whl", "bdist_wheel")]);
        assert_eq!(verdict(&intel_mac, TargetOs::Macos), Verdict::OtherSystemsOnly, "Apple silicon needs arm64");
    }

    #[test]
    fn pins_are_used_and_duplicates_dropped() {
        let deps = vec![
            ("torch".into(), "==2.4.1".into(), "requirements.txt".into()),
            ("torch".into(), ">=2".into(), "pyproject.toml".into()),
            ("numpy".into(), ">=1.26".into(), "pyproject.toml".into()),
        ];
        assert_eq!(unique_deps(&deps), vec![("torch".into(), Some("2.4.1".into())), ("numpy".into(), None)]);
    }
}
