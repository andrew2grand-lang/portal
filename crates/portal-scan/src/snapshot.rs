//! Snapshot: what the project is and how it runs here, read statically.
//! Nothing is executed except `python --version` / `node --version`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::parse;
use crate::report::{
    Ecosystem, LeftOut, LeftOutItem, Package, PackageSet, RunCommand, Runtime, SecretFile, Service, Support,
};
use crate::walk::{FileEntry, Inventory, is_os_junk};

/// Everything the rules and the report need about the project as a whole.
#[derive(Debug, Default)]
pub struct Snapshot {
    pub ecosystems: Vec<Ecosystem>,
    pub runtimes: Vec<Runtime>,
    pub packages: Vec<PackageSet>,
    pub run_commands: Vec<RunCommand>,
    pub left_out: LeftOut,
    pub secrets: Vec<SecretFile>,
    pub services: Vec<Service>,
    /// Python dependencies declared anywhere: (name, version spec as written, file).
    pub python_deps: Vec<(String, String, String)>,
    /// Node dependencies declared in package.json files.
    pub node_deps: Vec<(String, String, String)>,
    /// Lower bound of requires-python, as (major, minor), when stated.
    pub python_floor: Option<(u32, u32)>,
    pub requires_python: Option<(String, String)>,
    /// package.json scripts: (file, name, command).
    pub npm_scripts: Vec<(String, String, String)>,
    /// Docker images: (file, line, image, platform).
    pub docker_images: Vec<(String, u32, String, Option<String>)>,
    /// Native project markers: (kind, file).
    pub native: Vec<(&'static str, String)>,
}

const MAX_PACKAGES_LISTED: usize = 200;

fn name_of(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

fn ext_of(rel: &str) -> String {
    name_of(rel).rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

fn read_text(f: &FileEntry) -> Option<String> {
    if f.size > 4 * 1024 * 1024 {
        return None;
    }
    let bytes = std::fs::read(&f.abs).ok()?;
    parse::is_text(&bytes).then(|| String::from_utf8_lossy(&bytes).into_owned())
}

pub fn snapshot(root: &Path, inv: &Inventory, host_versions: bool) -> Snapshot {
    let mut s = Snapshot::default();
    let files = &inv.files;
    let by_name =
        |pred: &dyn Fn(&str) -> bool| -> Vec<&FileEntry> { files.iter().filter(|f| pred(name_of(&f.rel))).collect() };

    // ---- ecosystems -------------------------------------------------------
    let count_ext = |exts: &[&str]| files.iter().filter(|f| exts.contains(&ext_of(&f.rel).as_str())).count() as u64;
    let evidence = |names: &[&str]| -> Vec<String> {
        files
            .iter()
            .filter(|f| {
                let n = name_of(&f.rel);
                names
                    .iter()
                    .any(|p| if let Some(prefix) = p.strip_suffix('*') { n.starts_with(prefix) } else { n == *p })
            })
            .map(|f| f.rel.clone())
            .take(8)
            .collect()
    };
    let mut add = |kind: &str, name: &str, ev: Vec<String>, n: u64, support: Support| {
        if !ev.is_empty() || n > 0 {
            s.ecosystems.push(Ecosystem { kind: kind.into(), name: name.into(), evidence: ev, files: n, support });
        }
    };
    let py_ev = evidence(&[
        "pyproject.toml",
        "requirements*",
        "uv.lock",
        "poetry.lock",
        "Pipfile",
        "Pipfile.lock",
        "setup.py",
        "setup.cfg",
        "environment.yml",
        "environment.yaml",
        ".python-version",
    ]);
    let notebooks = count_ext(&["ipynb"]);
    add("python", "Python", py_ev, count_ext(&["py", "pyw"]) + notebooks, Support::Deep);
    let node_ev = evidence(&[
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lock*",
        "tsconfig.json",
        ".nvmrc",
    ]);
    add(
        "node",
        "JavaScript / TypeScript",
        node_ev,
        count_ext(&["js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx"]),
        Support::Deep,
    );
    let shells = count_ext(&["sh", "bash", "zsh", "command"]);
    let win_scripts = count_ext(&["ps1", "bat", "cmd"]);
    add("shell", "Shell scripts", vec![], shells, Support::Deep);
    add("windows-scripts", "Windows scripts", vec![], win_scripts, Support::Deep);
    add(
        "docker",
        "Docker",
        evidence(&["Dockerfile*", "docker-compose*", "compose.yml", "compose.yaml", "Containerfile"]),
        0,
        Support::Deep,
    );
    for (kind, name, names, exts) in [
        ("swift", "Swift / Xcode", &["Package.swift"][..], &["swift"][..]),
        ("dotnet", "C# / .NET", &[][..], &["cs", "csproj", "sln", "xaml", "vb", "fs"][..]),
        ("cpp", "C / C++", &["CMakeLists.txt"][..], &["c", "cc", "cpp", "cxx", "h", "hpp", "vcxproj"][..]),
        (
            "java",
            "Java / Kotlin",
            &["pom.xml", "build.gradle", "build.gradle.kts", "settings.gradle"][..],
            &["java", "kt", "kts"][..],
        ),
        ("go", "Go", &["go.mod"][..], &["go"][..]),
        ("rust", "Rust", &["Cargo.toml"][..], &["rs"][..]),
        ("ruby", "Ruby", &["Gemfile"][..], &["rb"][..]),
        ("php", "PHP", &["composer.json"][..], &["php"][..]),
    ] {
        add(kind, name, evidence(names), count_ext(exts), Support::Summary);
    }
    // Xcode projects are folders: their project.pbxproj shows them.
    for f in files {
        if f.rel.ends_with(".xcodeproj/project.pbxproj") {
            s.native.push(("xcode", f.rel.trim_end_matches("/project.pbxproj").to_owned()));
        }
        if f.rel.ends_with(".xcworkspace/contents.xcworkspacedata") {
            s.native.push(("xcode", f.rel.trim_end_matches("/contents.xcworkspacedata").to_owned()));
        }
    }
    if !s.native.is_empty() && !s.ecosystems.iter().any(|e| e.kind == "swift") {
        s.ecosystems.push(Ecosystem {
            kind: "swift".into(),
            name: "Swift / Xcode".into(),
            evidence: s.native.iter().map(|(_, f)| f.clone()).take(4).collect(),
            files: 0,
            support: Support::Summary,
        });
    }
    for f in files {
        let ext = ext_of(&f.rel);
        if ext == "csproj" || ext == "vbproj" {
            let text = read_text(f).unwrap_or_default();
            if text.contains("<UseWPF>true") {
                s.native.push(("wpf", f.rel.clone()));
            } else if text.contains("<UseWindowsForms>true") {
                s.native.push(("winforms", f.rel.clone()));
            } else if text.contains("Microsoft.NET.Sdk.WindowsDesktop") || text.contains("-windows</TargetFramework") {
                s.native.push(("windows-desktop", f.rel.clone()));
            }
        }
        if ext == "vcxproj" {
            s.native.push(("vcxproj", f.rel.clone()));
        }
    }

    // ---- runtimes ---------------------------------------------------------
    let mut py_req: Option<(String, String)> = None;
    for f in by_name(&|n| n == "pyproject.toml") {
        if let Some(doc) = read_text(f).and_then(|t| t.parse::<toml::Table>().ok()) {
            let spec =
                doc.get("project").and_then(|p| p.get("requires-python")).and_then(|v| v.as_str()).or_else(|| {
                    doc.get("tool")
                        .and_then(|t| t.get("poetry"))
                        .and_then(|p| p.get("dependencies"))
                        .and_then(|d| d.get("python"))
                        .and_then(|v| v.as_str())
                });
            if let Some(spec) = spec {
                py_req.get_or_insert((spec.to_owned(), f.rel.clone()));
            }
            collect_pyproject(&doc, &f.rel, &mut s);
        }
    }
    for f in by_name(&|n| n == ".python-version" || n == "runtime.txt") {
        if let Some(t) = read_text(f) {
            let v = t.lines().next().unwrap_or("").trim().trim_start_matches("python-").to_owned();
            if !v.is_empty() {
                py_req.get_or_insert((v, f.rel.clone()));
            }
        }
    }
    s.python_floor = py_req.as_ref().and_then(|(spec, _)| python_floor(spec));
    s.requires_python = py_req.clone();
    let has_python = s.ecosystems.iter().any(|e| e.kind == "python");
    if has_python {
        s.runtimes.push(Runtime {
            kind: "python".into(),
            requested: py_req.as_ref().map(|(v, _)| v.clone()),
            requested_from: py_req.map(|(_, f)| f),
            installed_here: host_versions.then(|| host_version(&["python3", "python"])).flatten(),
        });
    }
    let mut node_req: Option<(String, String)> = None;
    for f in by_name(&|n| n == "package.json") {
        if let Some(doc) = read_text(f).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
            if let Some(v) = doc.pointer("/engines/node").and_then(|v| v.as_str()) {
                node_req.get_or_insert((v.to_owned(), f.rel.clone()));
            }
            collect_package_json(&doc, &f.rel, &mut s);
        }
    }
    for f in by_name(&|n| n == ".nvmrc" || n == ".node-version") {
        if let Some(t) = read_text(f) {
            let v = t.trim().to_owned();
            if !v.is_empty() {
                node_req.get_or_insert((v, f.rel.clone()));
            }
        }
    }
    if s.ecosystems.iter().any(|e| e.kind == "node") {
        s.runtimes.push(Runtime {
            kind: "node".into(),
            requested: node_req.as_ref().map(|(v, _)| v.clone()),
            requested_from: node_req.map(|(_, f)| f),
            installed_here: host_versions.then(|| host_version(&["node"])).flatten(),
        });
    }

    // ---- Python dependencies (requirements, Pipfile, environment.yml) -----
    for f in by_name(&|n| n.starts_with("requirements") && n.ends_with(".txt")) {
        for (name, spec) in read_text(f).map(|t| parse_requirements(&t)).unwrap_or_default() {
            s.python_deps.push((name, spec, f.rel.clone()));
        }
    }
    for f in by_name(&|n| n == "Pipfile") {
        if let Some(doc) = read_text(f).and_then(|t| t.parse::<toml::Table>().ok())
            && let Some(pkgs) = doc.get("packages").and_then(|p| p.as_table())
        {
            for (name, v) in pkgs {
                let spec = v.as_str().unwrap_or("*").to_owned();
                s.python_deps.push((normalise(name), spec, f.rel.clone()));
            }
        }
    }
    for f in by_name(&|n| n == "environment.yml" || n == "environment.yaml") {
        if let Some(text) = read_text(f) {
            for (name, spec) in parse_conda_env(&text) {
                s.python_deps.push((name, spec, f.rel.clone()));
            }
        }
    }

    // ---- installed packages, read statically -------------------------------
    let py_lock = ["uv.lock", "poetry.lock", "Pipfile.lock", "pdm.lock", "conda-lock.yml"]
        .iter()
        .find_map(|n| files.iter().find(|f| name_of(&f.rel) == *n).map(|f| f.rel.clone()))
        .or_else(|| {
            // A fully pinned requirements file (pip freeze) is a lock too.
            by_name(&|n| n.starts_with("requirements") && n.ends_with(".txt")).into_iter().find_map(|f| {
                let reqs = read_text(f).map(|t| parse_requirements(&t)).unwrap_or_default();
                (!reqs.is_empty() && reqs.iter().all(|(_, spec)| spec.starts_with("=="))).then(|| f.rel.clone())
            })
        });
    let venv = inv.heavy.iter().find(|h| h.kind == "pythonEnv");
    let installed_py = venv.map(|v| read_dist_infos(&v.abs)).unwrap_or_default();
    let declared_py: BTreeSet<&str> = s.python_deps.iter().map(|(n, _, _)| n.as_str()).collect();
    if has_python && (!installed_py.is_empty() || !declared_py.is_empty()) {
        let items: Vec<Package> = if installed_py.is_empty() {
            s.python_deps
                .iter()
                .map(|(n, v, _)| Package { name: n.clone(), version: (!v.is_empty()).then(|| v.clone()) })
                .collect()
        } else {
            installed_py.iter().map(|(n, v)| Package { name: n.clone(), version: Some(v.clone()) }).collect()
        };
        s.packages.push(PackageSet {
            kind: "python".into(),
            source: venv.map(|v| v.rel.clone()).unwrap_or_else(|| "manifests".into()),
            lockfile: py_lock,
            declared: declared_py.len() as u32,
            installed: installed_py.len() as u32,
            items: items.into_iter().take(MAX_PACKAGES_LISTED).collect(),
        });
    }
    let node_lock =
        ["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lock", "bun.lockb", "npm-shrinkwrap.json"]
            .iter()
            .find_map(|n| files.iter().find(|f| name_of(&f.rel) == *n).map(|f| f.rel.clone()));
    let modules = inv.heavy.iter().find(|h| h.kind == "nodeModules" && !h.rel.contains('/'));
    let installed_node = modules.map(|m| read_node_modules(&m.abs)).unwrap_or_default();
    let declared_node: BTreeSet<&str> = s.node_deps.iter().map(|(n, _, _)| n.as_str()).collect();
    if !declared_node.is_empty() || !installed_node.is_empty() {
        let items: Vec<Package> = if installed_node.is_empty() {
            s.node_deps.iter().map(|(n, v, _)| Package { name: n.clone(), version: Some(v.clone()) }).collect()
        } else {
            installed_node.iter().map(|(n, v)| Package { name: n.clone(), version: Some(v.clone()) }).collect()
        };
        s.packages.push(PackageSet {
            kind: "node".into(),
            source: modules.map(|m| m.rel.clone()).unwrap_or_else(|| "package.json".into()),
            lockfile: node_lock,
            declared: declared_node.len() as u32,
            installed: installed_node.len() as u32,
            items: items.into_iter().take(MAX_PACKAGES_LISTED).collect(),
        });
    }

    // ---- how it's run -------------------------------------------------------
    for (file, name, command) in s.npm_scripts.clone() {
        s.run_commands.push(RunCommand { kind: "npm-script".into(), name, command, file });
    }
    for f in by_name(&|n| n == "Makefile" || n == "makefile" || n == "GNUmakefile") {
        for target in read_text(f).map(|t| make_targets(&t)).unwrap_or_default() {
            s.run_commands.push(RunCommand {
                kind: "make".into(),
                name: target.clone(),
                command: format!("make {target}"),
                file: f.rel.clone(),
            });
        }
    }
    for f in by_name(&|n| n == "Procfile") {
        for line in read_text(f).unwrap_or_default().lines() {
            if let Some((name, cmd)) = line.split_once(':')
                && !name.trim().is_empty()
                && !name.trim_start().starts_with('#')
            {
                s.run_commands.push(RunCommand {
                    kind: "procfile".into(),
                    name: name.trim().into(),
                    command: cmd.trim().into(),
                    file: f.rel.clone(),
                });
            }
        }
    }
    for f in files.iter().filter(|f| f.rel.ends_with(".vscode/launch.json")) {
        if let Some(doc) =
            read_text(f).and_then(|t| serde_json::from_str::<serde_json::Value>(&strip_json_comments(&t)).ok())
        {
            for c in doc.get("configurations").and_then(|c| c.as_array()).into_iter().flatten() {
                let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("launch").to_owned();
                let target = c
                    .get("program")
                    .or_else(|| c.get("module"))
                    .and_then(|v| v.as_str())
                    .map(|p| p.replace("${workspaceFolder}/", "").replace("${workspaceFolder}\\", ""));
                if let Some(target) = target {
                    s.run_commands.push(RunCommand {
                        kind: "vscode".into(),
                        name,
                        command: target,
                        file: f.rel.clone(),
                    });
                }
            }
        }
    }
    for f in by_name(&|n| n.eq_ignore_ascii_case("readme.md") || n.eq_ignore_ascii_case("readme.markdown")) {
        for cmd in read_text(f).map(|t| readme_commands(&t)).unwrap_or_default() {
            s.run_commands.push(RunCommand {
                kind: "readme".into(),
                name: "README".into(),
                command: cmd,
                file: f.rel.clone(),
            });
        }
    }

    // ---- what won't be sent -------------------------------------------------
    for h in &inv.heavy {
        s.left_out.items.push(LeftOutItem {
            path: h.rel.clone(),
            kind: h.kind.into(),
            bytes: h.bytes,
            files: h.files,
            rebuildable: true,
        });
    }
    let (mut junk_bytes, mut junk_files) = (0u64, 0u64);
    let mut ignored_by_top: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for f in &inv.ignored {
        if is_os_junk(name_of(&f.rel)) {
            junk_bytes += f.size;
            junk_files += 1;
        } else {
            let top = f.rel.split('/').next().unwrap_or(&f.rel).to_owned();
            let e = ignored_by_top.entry(top).or_default();
            e.0 += f.size;
            e.1 += 1;
        }
    }
    for (path, (bytes, n)) in ignored_by_top {
        let dir = root.join(&path).is_dir();
        s.left_out.items.push(LeftOutItem {
            path: if dir { format!("{path}/") } else { path },
            kind: "gitignored".into(),
            bytes,
            files: n,
            rebuildable: false,
        });
    }
    if junk_files > 0 {
        s.left_out.items.push(LeftOutItem {
            path: "OS clutter (.DS_Store, Thumbs.db…)".into(),
            kind: "osJunk".into(),
            bytes: junk_bytes,
            files: junk_files,
            rebuildable: false,
        });
    }
    s.left_out.bytes = s.left_out.items.iter().map(|i| i.bytes).sum();
    s.left_out.files = s.left_out.items.iter().map(|i| i.files).sum();

    // ---- secrets (names only) and services ----------------------------------
    let all_files = inv.files.iter().chain(inv.ignored.iter());
    for f in all_files {
        if let Some(kind) = secret_kind(name_of(&f.rel)) {
            let keys = if kind == "env" { env_key_names(&f.abs) } else { vec![] };
            s.secrets.push(SecretFile { path: f.rel.clone(), kind: kind.into(), keys });
        }
    }
    s.secrets.sort_by(|a, b| a.path.cmp(&b.path));
    collect_services(files, &mut s);
    s
}

/// Lower bound of a requires-python spec ("&gt;=3.11,&lt;4" → (3, 11); "3.12" → (3, 12)).
pub fn python_floor(spec: &str) -> Option<(u32, u32)> {
    spec.split(',')
        .filter_map(|part| {
            let p = part.trim();
            let v = p
                .strip_prefix(">=")
                .or_else(|| p.strip_prefix("~="))
                .or_else(|| p.strip_prefix("=="))
                .or_else(|| p.strip_prefix('^'))
                .or_else(|| p.strip_prefix('~'))
                .or_else(|| p.chars().next().filter(char::is_ascii_digit).map(|_| p))?;
            let mut nums = v.trim().split('.').map(|n| n.trim_end_matches('*').parse::<u32>());
            match (nums.next(), nums.next()) {
                (Some(Ok(a)), Some(Ok(b))) => Some((a, b)),
                (Some(Ok(a)), _) => Some((a, 0)),
                _ => None,
            }
        })
        .max()
}

fn host_version(commands: &[&str]) -> Option<String> {
    for cmd in commands {
        let child = Command::new(cmd)
            .arg("--version")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();
        let Ok(mut child) = child else { continue };
        let started = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if started.elapsed() > Duration::from_secs(3) => {
                    let _ = child.kill();
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => break,
            }
        }
        if let Ok(out) = child.wait_with_output() {
            let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
            let v =
                text.split_whitespace().find(|w| w.trim_start_matches('v').starts_with(|c: char| c.is_ascii_digit()));
            if let Some(v) = v.filter(|_| out.status.success()) {
                return Some(v.trim_start_matches('v').to_owned());
            }
        }
    }
    None
}

/// PEP 503 normalised name.
pub fn normalise(name: &str) -> String {
    let mut out = String::new();
    let mut last_sep = false;
    for c in name.trim().chars() {
        if matches!(c, '-' | '_' | '.') {
            if !last_sep {
                out.push('-');
            }
            last_sep = true;
        } else {
            out.push(c.to_ascii_lowercase());
            last_sep = false;
        }
    }
    out
}

/// (name, spec) from a PEP 508 requirement ("torch[cuda]>=2.1; python_version>'3.9'").
pub fn parse_requirement(line: &str) -> Option<(String, String)> {
    let line = line.split('#').next()?.trim();
    if line.is_empty()
        || line.starts_with('-')
        || line.contains("://")
        || line.starts_with('.')
        || line.starts_with('/')
    {
        return None;
    }
    let line = line.split(';').next()?.trim();
    let end = line.find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))).unwrap_or(line.len());
    let name = &line[..end];
    if name.is_empty() {
        return None;
    }
    let mut rest = line[end..].trim();
    if rest.starts_with('[') {
        rest = rest.split_once(']').map(|(_, r)| r.trim()).unwrap_or("");
    }
    let rest = rest.trim_start_matches('(').trim_end_matches(')').trim();
    Some((normalise(name), rest.to_owned()))
}

pub fn parse_requirements(text: &str) -> Vec<(String, String)> {
    text.lines().filter_map(parse_requirement).collect()
}

fn collect_pyproject(doc: &toml::Table, rel: &str, s: &mut Snapshot) {
    let project = doc.get("project");
    for dep in project.and_then(|p| p.get("dependencies")).and_then(|d| d.as_array()).into_iter().flatten() {
        if let Some((name, spec)) = dep.as_str().and_then(parse_requirement) {
            s.python_deps.push((name, spec, rel.to_owned()));
        }
    }
    if let Some(groups) = project.and_then(|p| p.get("optional-dependencies")).and_then(|d| d.as_table()) {
        for list in groups.values().filter_map(|v| v.as_array()) {
            for dep in list {
                if let Some((name, spec)) = dep.as_str().and_then(parse_requirement) {
                    s.python_deps.push((name, spec, rel.to_owned()));
                }
            }
        }
    }
    let poetry = doc.get("tool").and_then(|t| t.get("poetry"));
    if let Some(deps) = poetry.and_then(|p| p.get("dependencies")).and_then(|d| d.as_table()) {
        for (name, v) in deps {
            if name == "python" {
                continue;
            }
            let spec =
                v.as_str().map(str::to_owned).or_else(|| v.get("version").and_then(|x| x.as_str()).map(str::to_owned));
            s.python_deps.push((normalise(name), spec.unwrap_or_default(), rel.to_owned()));
        }
    }
    let scripts = project
        .and_then(|p| p.get("scripts"))
        .and_then(|d| d.as_table())
        .or_else(|| poetry.and_then(|p| p.get("scripts")).and_then(|d| d.as_table()));
    for (name, target) in scripts.into_iter().flatten() {
        s.run_commands.push(RunCommand {
            kind: "pyproject-script".into(),
            name: name.clone(),
            command: target.as_str().unwrap_or_default().to_owned(),
            file: rel.to_owned(),
        });
    }
}

fn collect_package_json(doc: &serde_json::Value, rel: &str, s: &mut Snapshot) {
    for key in ["dependencies", "devDependencies", "optionalDependencies"] {
        if let Some(map) = doc.get(key).and_then(|d| d.as_object()) {
            for (name, v) in map {
                s.node_deps.push((name.clone(), v.as_str().unwrap_or_default().to_owned(), rel.to_owned()));
            }
        }
    }
    if let Some(map) = doc.get("scripts").and_then(|d| d.as_object()) {
        for (name, cmd) in map {
            if let Some(cmd) = cmd.as_str() {
                s.npm_scripts.push((rel.to_owned(), name.clone(), cmd.to_owned()));
            }
        }
    }
}

/// Conda environment.yml: conda packages and the nested pip list.
fn parse_conda_env(text: &str) -> Vec<(String, String)> {
    let Ok(docs) = yaml_rust2::YamlLoader::load_from_str(text) else { return vec![] };
    let mut out = Vec::new();
    for doc in docs {
        for dep in doc["dependencies"].as_vec().into_iter().flatten() {
            if let Some(s) = dep.as_str() {
                let s = s.replace('=', "==").replace("====", "==");
                if let Some(r) = parse_requirement(&s) {
                    out.push(r);
                }
            } else if let Some(pips) = dep["pip"].as_vec() {
                out.extend(pips.iter().filter_map(|p| p.as_str()).filter_map(parse_requirement));
            }
        }
    }
    out
}

/// name → version from `*.dist-info/METADATA` in a virtual environment.
fn read_dist_infos(venv: &Path) -> Vec<(String, String)> {
    let mut site_dirs = Vec::new();
    for lib in ["lib", "Lib"] {
        let lib_dir = venv.join(lib);
        if !not_a_link(&lib_dir) {
            continue;
        }
        if lib_dir.join("site-packages").is_dir() {
            site_dirs.push(lib_dir.join("site-packages"));
        }
        for e in std::fs::read_dir(&lib_dir).into_iter().flatten().flatten() {
            if e.file_name().to_string_lossy().starts_with("python") && e.path().join("site-packages").is_dir() {
                site_dirs.push(e.path().join("site-packages"));
            }
        }
    }
    site_dirs.retain(|d| not_a_link(d) && d.parent().is_some_and(not_a_link));
    let mut out = BTreeMap::new();
    for dir in site_dirs {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let metadata = e.path().join("METADATA");
            if !name.ends_with(".dist-info") || !not_a_link(&e.path()) || !not_a_link(&metadata) {
                continue;
            }
            let meta = std::fs::read_to_string(metadata).unwrap_or_default();
            let field = |key: &str| meta.lines().find_map(|l| l.strip_prefix(key).map(|v| v.trim().to_owned()));
            if let (Some(n), Some(v)) = (field("Name:"), field("Version:")) {
                out.insert(normalise(&n), v);
            }
        }
    }
    out.into_iter().collect()
}

/// Top-level packages in node_modules (scoped ones too), from their package.json.
fn read_node_modules(dir: &Path) -> Vec<(String, String)> {
    let mut out = BTreeMap::new();
    let mut read = |p: &Path| {
        if !not_a_link(p) || !not_a_link(&p.join("package.json")) {
            return;
        }
        if let Ok(t) = std::fs::read_to_string(p.join("package.json"))
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&t)
            && let (Some(n), Some(ver)) =
                (v.get("name").and_then(|x| x.as_str()), v.get("version").and_then(|x| x.as_str()))
        {
            out.insert(n.to_owned(), ver.to_owned());
        }
    };
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if name.starts_with('@') {
            if !not_a_link(&e.path()) {
                continue;
            }
            for inner in std::fs::read_dir(e.path()).into_iter().flatten().flatten() {
                read(&inner.path());
            }
        } else {
            read(&e.path());
        }
    }
    out.into_iter().collect()
}

/// Targets a person would run: not variables, special targets or pattern rules.
fn make_targets(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with(['\t', ' ', '#', '.']) {
            continue;
        }
        let Some((head, rest)) = line.split_once(':') else { continue };
        if rest.starts_with('=') || head.contains(['=', '%', '$']) {
            continue;
        }
        for t in head.split_whitespace() {
            if !out.contains(&t.to_owned()) {
                out.push(t.to_owned());
            }
        }
    }
    out.truncate(20);
    out
}

/// Commands from fenced shell blocks in a README that look like running the project.
fn readme_commands(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    let mut shell_block = false;
    for line in text.lines() {
        let t = line.trim();
        if let Some(lang) = t.strip_prefix("```") {
            if in_block {
                in_block = false;
            } else {
                in_block = true;
                let lang = lang.trim().to_ascii_lowercase();
                shell_block = lang.is_empty()
                    || matches!(
                        lang.as_str(),
                        "sh" | "bash" | "shell" | "console" | "zsh" | "powershell" | "ps1" | "cmd" | "bat"
                    );
            }
            continue;
        }
        if !(in_block && shell_block) {
            continue;
        }
        let cmd = t.trim_start_matches("$ ").trim_start_matches("> ").trim_start_matches("PS> ");
        let first = cmd.split_whitespace().next().unwrap_or("");
        let runs = matches!(
            first,
            "python"
                | "python3"
                | "py"
                | "uv"
                | "poetry"
                | "pipenv"
                | "node"
                | "npm"
                | "npx"
                | "pnpm"
                | "yarn"
                | "bun"
                | "make"
                | "docker"
                | "bash"
                | "sh"
                | "flask"
                | "uvicorn"
                | "streamlit"
                | "jupyter"
        ) || first.starts_with("./")
            || first.starts_with(".\\")
            || [".bat", ".cmd", ".ps1", ".sh"].iter().any(|e| first.to_ascii_lowercase().ends_with(e));
        if runs && out.len() < 20 && !out.contains(&cmd.to_owned()) {
            out.push(cmd.to_owned());
        }
    }
    out
}

/// .vscode/launch.json allows // comments.
fn strip_json_comments(text: &str) -> String {
    text.lines()
        .map(|l| {
            let t = l.trim_start();
            if t.starts_with("//") { "" } else { l }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Files that hold secrets, by name. Examples and templates don't count.
pub fn secret_kind(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    let template =
        ["example", "sample", "template", "dist", "defaults"].iter().any(|t| lower.ends_with(&format!(".{t}")));
    if (lower == ".env" || lower.starts_with(".env.") || lower.ends_with(".env")) && !template {
        return Some("env");
    }
    let key_ext = [".pem", ".key", ".p12", ".pfx", ".keystore", ".jks", ".p8", ".ppk", ".asc"];
    if key_ext.iter().any(|e| lower.ends_with(e))
        || lower.starts_with("id_rsa")
        || lower.starts_with("id_ed25519")
        || lower.starts_with("id_ecdsa")
    {
        if lower.ends_with(".pub") {
            return None;
        }
        return Some("key");
    }
    if (lower.starts_with("credentials")
        || lower.starts_with("service-account")
        || lower.contains("secret") && lower.ends_with(".json"))
        && !template
        && !lower.ends_with(".md")
        && !lower.ends_with(".py")
        && !lower.ends_with(".ts")
        && !lower.ends_with(".js")
    {
        return Some("credentials");
    }
    if lower == ".npmrc" || lower == ".pypirc" || lower == ".netrc" {
        return Some("credentials");
    }
    None
}

/// Variable names in a .env file. Values are read past and never kept,
/// including quoted values that run over several lines (keys, certificates).
fn env_key_names(path: &Path) -> Vec<String> {
    let Ok(bytes) = std::fs::read(path) else { return vec![] };
    let text = String::from_utf8_lossy(&bytes);
    let mut names = Vec::new();
    let mut open_quote: Option<char> = None;
    for raw in text.lines() {
        if let Some(q) = open_quote {
            if closes(raw, q) {
                open_quote = None;
            }
            continue;
        }
        let line = raw.trim().trim_start_matches("export ").trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else { continue };
        let name = name.trim();
        if name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !names.contains(&name.to_owned())
        {
            names.push(name.to_owned());
        }
        let value = value.trim_start();
        if let Some(q) = value.chars().next().filter(|c| matches!(c, '"' | '\'' | '`'))
            && !closes(&value[1..], q)
        {
            open_quote = Some(q);
        }
    }
    names
}

/// The quote `q` appears unescaped in `s`.
fn closes(s: &str, q: char) -> bool {
    let mut escaped = false;
    for c in s.chars() {
        if c == q && !escaped {
            return true;
        }
        escaped = c == '\\' && !escaped;
    }
    false
}

/// A real folder or file in the project: never a link (it could point anywhere).
fn not_a_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| !m.file_type().is_symlink())
}

/// Databases and other services, from compose files and dependencies.
fn collect_services(files: &[FileEntry], s: &mut Snapshot) {
    const KNOWN: &[(&str, &str, &str)] = &[
        ("postgres", "PostgreSQL", "postgres"),
        ("postgis", "PostgreSQL", "postgres"),
        ("mysql", "MySQL", "mysql"),
        ("mariadb", "MariaDB", "mariadb"),
        ("mongo", "MongoDB", "mongodb"),
        ("redis", "Redis", "redis"),
        ("valkey", "Valkey", "redis"),
        ("rabbitmq", "RabbitMQ", "rabbitmq"),
        ("elasticsearch", "Elasticsearch", "elasticsearch"),
        ("opensearch", "OpenSearch", "elasticsearch"),
        ("kafka", "Kafka", "kafka"),
        ("minio", "MinIO", "minio"),
        ("memcached", "Memcached", "memcached"),
        ("ollama", "Ollama", "ollama"),
        ("qdrant", "Qdrant", "qdrant"),
        ("chroma", "Chroma", "chroma"),
    ];
    for f in files {
        let n = name_of(&f.rel).to_ascii_lowercase();
        let is_compose = (n.starts_with("docker-compose") || n.starts_with("compose"))
            && (n.ends_with(".yml") || n.ends_with(".yaml"));
        let is_dockerfile =
            n == "dockerfile" || n.starts_with("dockerfile.") || n.ends_with(".dockerfile") || n == "containerfile";
        if is_compose {
            let Some(text) = read_text(f) else { continue };
            let Ok(docs) = yaml_rust2::YamlLoader::load_from_str(&text) else { continue };
            for doc in docs {
                let Some(services) = doc["services"].as_hash() else { continue };
                for (name, svc) in services {
                    let image = svc["image"].as_str().unwrap_or("").to_owned();
                    let platform = svc["platform"].as_str().map(str::to_owned);
                    let line = text
                        .lines()
                        .position(|l| !image.is_empty() && l.contains(&image))
                        .map(|i| i as u32 + 1)
                        .unwrap_or(1);
                    if !image.is_empty() {
                        s.docker_images.push((f.rel.clone(), line, image.clone(), platform));
                    }
                    let svc_name = name.as_str().unwrap_or("service").to_owned();
                    let hay = format!("{} {}", image.to_ascii_lowercase(), svc_name.to_ascii_lowercase());
                    if let Some((_, label, kind)) = KNOWN.iter().find(|(key, _, _)| hay.contains(key)) {
                        s.services.push(Service { kind: (*kind).into(), name: (*label).into(), from: f.rel.clone() });
                    }
                }
            }
        } else if is_dockerfile {
            let Some(text) = read_text(f) else { continue };
            for (i, line) in text.lines().enumerate() {
                let t = line.trim();
                let upper = t.to_ascii_uppercase();
                if let Some(rest) = upper.strip_prefix("FROM ").map(|_| &t[5..]) {
                    let mut platform = None;
                    let mut image = None;
                    for word in rest.split_whitespace() {
                        if let Some(p) = word.strip_prefix("--platform=") {
                            platform = Some(p.to_owned());
                        } else if image.is_none() {
                            image = Some(word.to_owned());
                        }
                    }
                    if let Some(image) = image.filter(|i| !i.eq_ignore_ascii_case("scratch")) {
                        s.docker_images.push((f.rel.clone(), i as u32 + 1, image, platform));
                    }
                }
            }
        }
    }
    // Services a Python or Node project talks to, by client library.
    let deps: Vec<String> =
        s.python_deps.iter().chain(s.node_deps.iter()).map(|(n, _, _)| n.to_ascii_lowercase()).collect();
    for (dep, label, kind) in [
        ("psycopg2", "PostgreSQL", "postgres"),
        ("psycopg2-binary", "PostgreSQL", "postgres"),
        ("psycopg", "PostgreSQL", "postgres"),
        ("asyncpg", "PostgreSQL", "postgres"),
        ("pg", "PostgreSQL", "postgres"),
        ("mysqlclient", "MySQL", "mysql"),
        ("pymysql", "MySQL", "mysql"),
        ("mysql2", "MySQL", "mysql"),
        ("pymongo", "MongoDB", "mongodb"),
        ("mongoose", "MongoDB", "mongodb"),
        ("redis", "Redis", "redis"),
        ("ioredis", "Redis", "redis"),
        ("celery", "a task queue broker (Redis or RabbitMQ)", "broker"),
        ("ollama", "Ollama", "ollama"),
    ] {
        if deps.iter().any(|d| d == dep) && !s.services.iter().any(|x| x.kind == kind) {
            s.services.push(Service { kind: kind.into(), name: label.into(), from: format!("dependency {dep}") });
        }
    }
    s.services.dedup_by(|a, b| a.kind == b.kind);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_lines_parse_to_normalised_names() {
        assert_eq!(
            parse_requirement("Torch[cuda]>=2.1 ; python_version > '3.9'"),
            Some(("torch".into(), ">=2.1".into()))
        );
        assert_eq!(parse_requirement("python_dotenv==1.0.1  # env"), Some(("python-dotenv".into(), "==1.0.1".into())));
        assert_eq!(parse_requirement("-r base.txt"), None);
        assert_eq!(parse_requirement("git+https://github.com/x/y.git"), None);
        assert_eq!(parse_requirement("numpy"), Some(("numpy".into(), String::new())));
    }

    #[test]
    fn the_python_floor_is_the_lowest_version_asked_for() {
        assert_eq!(python_floor(">=3.11,<4"), Some((3, 11)));
        assert_eq!(python_floor("^3.10"), Some((3, 10)));
        assert_eq!(python_floor("3.12.4"), Some((3, 12)));
        assert_eq!(python_floor("<3.13"), None);
    }

    #[test]
    fn secrets_are_known_by_name_and_templates_are_not_secrets() {
        for name in [
            ".env",
            ".env.local",
            "prod.env",
            "server.pem",
            "id_rsa",
            "credentials.json",
            "service-account.json",
            ".npmrc",
        ] {
            assert!(secret_kind(name).is_some(), "{name}");
        }
        for name in [".env.example", ".env.sample", "id_rsa.pub", "README.md", "env.py", "credentials.md"] {
            assert!(secret_kind(name).is_none(), "{name}");
        }
    }

    #[test]
    fn make_targets_skip_variables_and_pattern_rules() {
        let mk = "CC=gcc\n.PHONY: run\nrun: build\n\tpython main.py\n%.o: %.c\nbuild test:\n";
        assert_eq!(make_targets(mk), ["run", "build", "test"]);
    }

    #[test]
    fn readme_commands_come_from_shell_blocks_only() {
        let md = "Install:\n```bash\n$ uv sync\npython app.py --port 8000\necho hi\n```\n```python\npython = 3\n```\n";
        assert_eq!(readme_commands(md), ["uv sync", "python app.py --port 8000"]);
    }
}
