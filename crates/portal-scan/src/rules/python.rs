//! Python rules, on the tree-sitter syntax tree.

use std::collections::HashSet;

use tree_sitter::Node;

use super::{Ctx, Hit};
use crate::parse::{self, evidence, line, string_value, text, walk_nodes};
use crate::report::{Severity, State, TargetOs};

/// Unix-only modules → what to use on Windows.
const UNIX_MODULES: &[(&str, &str)] = &[
    ("fcntl", "portalocker (or msvcrt.locking)"),
    ("termios", "msvcrt, or a cross-platform library such as prompt_toolkit"),
    ("tty", "msvcrt, or prompt_toolkit"),
    ("pty", "pywinpty"),
    ("resource", "psutil"),
    ("pwd", "getpass.getuser() and Path.home()"),
    ("grp", "a Windows-aware library such as psutil"),
    ("curses", "the windows-curses package"),
    ("crypt", "passlib or bcrypt"),
    ("spwd", "a cross-platform auth library"),
    ("posix", "the os module"),
    ("syslog", "the logging module"),
    ("readline", "pyreadline3"),
];

/// Windows-only modules → what to use on a Mac.
const WINDOWS_MODULES: &[(&str, &str)] = &[
    ("winreg", "a settings file (json, plistlib) or the platformdirs package"),
    ("_winreg", "a settings file (json, plistlib) or the platformdirs package"),
    ("msvcrt", "termios and tty"),
    ("winsound", "a cross-platform audio library (simpleaudio, sounddevice)"),
    ("_winapi", "the os and subprocess modules"),
    ("win32api", "the Mac equivalents, chosen with a platform check"),
    ("win32con", "the Mac equivalents, chosen with a platform check"),
    ("win32gui", "a cross-platform GUI toolkit"),
    ("win32com", "AppleScript / osascript, or a cross-platform library"),
    ("win32process", "the subprocess and psutil modules"),
    ("win32file", "the os and pathlib modules"),
    ("win32event", "the threading module"),
    ("win32service", "launchd on a Mac"),
    ("win32serviceutil", "launchd on a Mac"),
    ("win32clipboard", "pyperclip"),
    ("win32print", "a cross-platform printing route (lp / CUPS)"),
    ("pythoncom", "a platform check around the Windows-only parts"),
    ("pywintypes", "a platform check around the Windows-only parts"),
    ("wmi", "psutil"),
    ("winshell", "pathlib and platformdirs"),
    ("pywinauto", "a Mac automation route (AppleScript, pyobjc)"),
    ("comtypes", "a platform check around the Windows-only parts"),
];

const MLX_MODULES: &[&str] = &["mlx", "mlx_lm", "mlx_whisper", "mlx_vlm", "mlx_embeddings"];

/// Libraries that need CUDA (state when imported at top level).
const CUDA_LIBRARIES: &[(&str, State)] = &[
    ("flash_attn", State::Blocker),
    ("cupy", State::Blocker),
    ("pycuda", State::Blocker),
    ("tensorrt", State::Blocker),
    ("triton", State::Blocker),
    ("xformers", State::Blocker),
    ("bitsandbytes", State::WillNeedAFix),
];

/// os.* functions missing on Windows → advice.
const UNIX_OS_API: &[(&str, &str)] = &[
    (
        "geteuid",
        "Check admin rights another way on Windows (ctypes.windll.shell32.IsUserAnAdmin), behind a platform check.",
    ),
    ("getuid", "Use getpass.getuser() for the user's name; skip the numeric id on Windows."),
    ("getgid", "Skip group ids on Windows, behind a platform check."),
    ("getegid", "Skip group ids on Windows, behind a platform check."),
    ("getgroups", "Skip group ids on Windows, behind a platform check."),
    ("setuid", "Skip switching users on Windows, behind a platform check."),
    ("setgid", "Skip switching groups on Windows, behind a platform check."),
    ("seteuid", "Skip switching users on Windows, behind a platform check."),
    ("setsid", "Start detached processes with subprocess and creationflags on Windows."),
    ("getpgid", "Use psutil for process groups, behind a platform check."),
    ("setpgid", "Use subprocess's start_new_session, which works on both."),
    ("getpgrp", "Use psutil for process groups, behind a platform check."),
    ("killpg", "Stop processes one by one (psutil), or use a Job Object on Windows."),
    ("fork", "Use multiprocessing or subprocess, which work on both."),
    ("forkpty", "Use pywinpty on Windows."),
    ("uname", "Use platform.uname(), which works on both."),
    ("wait3", "Use subprocess and Popen.wait()."),
    ("wait4", "Use subprocess and Popen.wait()."),
    ("WNOHANG", "Use subprocess and Popen.poll()."),
    ("mkfifo", "Use a named pipe through multiprocessing.connection or sockets."),
    ("mknod", "Skip device files on Windows."),
    ("chown", "Skip ownership changes on Windows, behind a platform check."),
    ("lchown", "Skip ownership changes on Windows, behind a platform check."),
    ("fchown", "Skip ownership changes on Windows, behind a platform check."),
    ("chroot", "There's no Windows equivalent; skip it there."),
    ("getloadavg", "Use psutil.getloadavg(), which works on both."),
    ("statvfs", "Use shutil.disk_usage(), which works on both."),
    ("fstatvfs", "Use shutil.disk_usage(), which works on both."),
    ("nice", "Use psutil's nice(), which works on both."),
    ("sched_getaffinity", "Use os.cpu_count() or psutil.Process().cpu_affinity()."),
    ("sched_setaffinity", "Use psutil.Process().cpu_affinity()."),
    ("sysconf", "Use os.cpu_count() or psutil."),
    ("openpty", "Use pywinpty on Windows."),
    ("ttyname", "Skip it on Windows, behind a platform check."),
    ("getpriority", "Use psutil's nice()."),
    ("setpriority", "Use psutil's nice()."),
    ("pipe2", "Use os.pipe()."),
    ("memfd_create", "Use tempfile."),
];

/// Signals Windows doesn't define.
const UNIX_SIGNALS: &[&str] = &[
    "SIGKILL",
    "SIGHUP",
    "SIGALRM",
    "SIGUSR1",
    "SIGUSR2",
    "SIGCHLD",
    "SIGQUIT",
    "SIGPIPE",
    "SIGSTOP",
    "SIGCONT",
    "SIGTSTP",
    "SIGTTIN",
    "SIGTTOU",
    "SIGWINCH",
    "SIGXCPU",
    "SIGPROF",
    "SIGVTALRM",
    "SIGIO",
    "SIGSYS",
    "SIGTRAP",
    "SIGURG",
    "alarm",
    "pause",
    "setitimer",
    "getitimer",
    "siginterrupt",
    "sigpending",
    "sigwait",
    "pthread_kill",
    "pthread_sigmask",
];

/// Functions that only exist on Windows → advice for a Mac.
const WINDOWS_API: &[(&str, &str, &str)] = &[
    ("os", "startfile", "Open files with subprocess.run([\"open\", path]) on a Mac, behind a platform check."),
    ("os", "add_dll_directory", "Skip it on a Mac, behind a platform check."),
    ("signal", "CTRL_C_EVENT", "Use signal.SIGINT on a Mac."),
    ("signal", "CTRL_BREAK_EVENT", "Use signal.SIGINT on a Mac."),
    ("signal", "SIGBREAK", "Use signal.SIGINT on a Mac."),
];

/// Environment variables only one OS sets → (target where it's missing, advice).
pub const ENV_VARS: &[(&str, TargetOs, &str)] = &[
    ("HOME", TargetOs::Windows, "Use Path.home(), which works on both."),
    ("USER", TargetOs::Windows, "Use getpass.getuser(), which works on both."),
    ("LOGNAME", TargetOs::Windows, "Use getpass.getuser(), which works on both."),
    ("SHELL", TargetOs::Windows, "Don't rely on a login shell; call the program directly."),
    ("TMPDIR", TargetOs::Windows, "Use tempfile.gettempdir(), which works on both."),
    ("DISPLAY", TargetOs::Windows, "Check for a display another way, or skip the check on Windows."),
    ("XDG_CONFIG_HOME", TargetOs::Windows, "Use the platformdirs package for config folders."),
    ("XDG_DATA_HOME", TargetOs::Windows, "Use the platformdirs package for data folders."),
    ("XDG_CACHE_HOME", TargetOs::Windows, "Use the platformdirs package for cache folders."),
    ("XDG_RUNTIME_DIR", TargetOs::Windows, "Use tempfile or platformdirs."),
    ("APPDATA", TargetOs::Macos, "Use the platformdirs package for app data folders."),
    ("LOCALAPPDATA", TargetOs::Macos, "Use the platformdirs package for app data folders."),
    ("USERPROFILE", TargetOs::Macos, "Use Path.home(), which works on both."),
    ("USERNAME", TargetOs::Macos, "Use getpass.getuser(), which works on both."),
    ("COMPUTERNAME", TargetOs::Macos, "Use platform.node(), which works on both."),
    ("PROGRAMFILES", TargetOs::Macos, "Look the program up with shutil.which(), which works on both."),
    ("PROGRAMDATA", TargetOs::Macos, "Use the platformdirs package."),
    ("SYSTEMROOT", TargetOs::Macos, "Skip it on a Mac, behind a platform check."),
    ("WINDIR", TargetOs::Macos, "Skip it on a Mac, behind a platform check."),
    ("COMSPEC", TargetOs::Macos, "Call programs directly with subprocess instead of through the shell."),
    ("HOMEDRIVE", TargetOs::Macos, "Use Path.home(), which works on both."),
    ("HOMEPATH", TargetOs::Macos, "Use Path.home(), which works on both."),
    ("TEMP", TargetOs::Macos, "Use tempfile.gettempdir(), which works on both."),
];

/// Commands one OS lacks: (command, missing on, advice).
pub const SHELL_COMMANDS: &[(&str, TargetOs, &str)] = &[
    ("ls", TargetOs::Windows, "List folders with os.listdir() or pathlib."),
    ("cat", TargetOs::Windows, "Read files with pathlib's read_text()."),
    ("grep", TargetOs::Windows, "Search in Python (re, or a loop over lines)."),
    ("rm", TargetOs::Windows, "Delete with os.remove() or shutil.rmtree()."),
    ("cp", TargetOs::Windows, "Copy with shutil.copy2() or shutil.copytree()."),
    ("mv", TargetOs::Windows, "Move with shutil.move()."),
    ("chmod", TargetOs::Windows, "Use os.chmod(), or skip it on Windows."),
    ("chown", TargetOs::Windows, "Skip ownership changes on Windows."),
    ("sed", TargetOs::Windows, "Edit text in Python (str.replace, re.sub)."),
    ("awk", TargetOs::Windows, "Process text in Python."),
    ("which", TargetOs::Windows, "Use shutil.which()."),
    ("touch", TargetOs::Windows, "Use Path.touch()."),
    ("ln", TargetOs::Windows, "Use os.symlink(), knowing Windows needs Developer Mode for links."),
    ("head", TargetOs::Windows, "Read the first lines in Python."),
    ("tail", TargetOs::Windows, "Read the last lines in Python."),
    ("open", TargetOs::Windows, "Use os.startfile() on Windows and open on a Mac, chosen with a platform check."),
    ("say", TargetOs::Windows, "Use a cross-platform text-to-speech library (pyttsx3)."),
    ("pbcopy", TargetOs::Windows, "Use pyperclip."),
    ("pbpaste", TargetOs::Windows, "Use pyperclip."),
    ("osascript", TargetOs::Windows, "Use the Windows equivalent behind a platform check."),
    ("xdg-open", TargetOs::Windows, "Use webbrowser.open() or os.startfile()."),
    ("sudo", TargetOs::Windows, "Ask for admin rights another way on Windows."),
    ("clear", TargetOs::Windows, "Use \"cls\" on Windows (os.system(\"cls\" if os.name == \"nt\" else \"clear\"))."),
    ("bash", TargetOs::Windows, "Call the program directly, or ship a PowerShell version."),
    ("sh", TargetOs::Windows, "Call the program directly, or ship a PowerShell version."),
    ("cmd", TargetOs::Macos, "Call the program directly; a Mac has no cmd.exe."),
    ("cmd.exe", TargetOs::Macos, "Call the program directly; a Mac has no cmd.exe."),
    ("powershell", TargetOs::Macos, "Call the program directly, or ship a shell script version."),
    ("start", TargetOs::Macos, "Use open on a Mac, chosen with a platform check."),
    ("del", TargetOs::Macos, "Delete with os.remove() or shutil.rmtree()."),
    ("copy", TargetOs::Macos, "Copy with shutil.copy2()."),
    ("move", TargetOs::Macos, "Move with shutil.move()."),
    ("dir", TargetOs::Macos, "List folders with os.listdir() or pathlib."),
    ("reg", TargetOs::Macos, "Use a settings file on a Mac."),
    ("wmic", TargetOs::Macos, "Use psutil."),
    ("taskkill", TargetOs::Macos, "Use psutil or Popen.terminate()."),
    ("cls", TargetOs::Macos, "Use \"clear\" on a Mac (os.system(\"cls\" if os.name == \"nt\" else \"clear\"))."),
    ("explorer", TargetOs::Macos, "Use open on a Mac."),
    ("notepad", TargetOs::Macos, "Open the file with the default app instead."),
    ("where", TargetOs::Macos, "Use shutil.which()."),
    ("robocopy", TargetOs::Macos, "Copy with shutil.copytree()."),
    ("xcopy", TargetOs::Macos, "Copy with shutil.copytree()."),
];

fn first_dotted(node: Node<'_>, src: &[u8]) -> String {
    text(node, src).split('.').next().unwrap_or("").trim().to_owned()
}

/// Inside `if sys.platform …`, `if os.name …`, `hasattr(…)`, a WINDOWS / MAC flag,
/// or a try block that catches ImportError / AttributeError / OSError / Exception.
fn guarded(node: Node<'_>, src: &[u8]) -> bool {
    let mut n = node.parent();
    let mut child = node;
    while let Some(p) = n {
        match p.kind() {
            "if_statement" | "elif_clause" | "conditional_expression" | "while_statement" => {
                let cond = p.child_by_field_name("condition").map(|c| text(c, src)).unwrap_or_else(|| text(p, src));
                let cond = if p.kind() == "conditional_expression" { text(p, src) } else { cond };
                if is_platform_check(cond) {
                    return true;
                }
            }
            "try_statement" => {
                // Only the try body is guarded, not the handlers.
                let in_body = p.child_by_field_name("body").is_some_and(|b| b.id() == child.id());
                if in_body {
                    let mut cursor = p.walk();
                    for clause in p.children(&mut cursor) {
                        if clause.kind() == "except_clause" {
                            let t = text(clause, src);
                            let first = t.lines().next().unwrap_or("");
                            if [
                                "ImportError",
                                "ModuleNotFoundError",
                                "AttributeError",
                                "OSError",
                                "Exception",
                                "NotImplementedError",
                            ]
                            .iter()
                            .any(|e| first.contains(e))
                                || first.trim_end_matches(':').trim() == "except"
                            {
                                return true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        child = p;
        n = p.parent();
    }
    false
}

/// A condition that picks an OS (or checks a feature exists): whole calls and
/// identifiers only, so `if count > 0` doesn't count.
pub fn is_platform_check(cond: &str) -> bool {
    let c = cond.to_ascii_lowercase();
    const CALLS: &[&str] = &[
        "sys.platform",
        "os.name",
        "platform.system",
        "platform.platform",
        "platform.mac_ver",
        "hasattr(",
        "is_available()",
        "shutil.which(",
    ];
    const WORDS: &[&str] = &[
        "windows",
        "is_windows",
        "iswindows",
        "on_windows",
        "is_win",
        "win32",
        "darwin",
        "is_mac",
        "macos",
        "is_macos",
        "on_mac",
        "is_posix",
        "is_linux",
    ];
    CALLS.iter().any(|k| c.contains(k))
        || c.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')).any(|w| WORDS.contains(&w))
}

fn in_function(node: Node<'_>) -> bool {
    let mut n = node.parent();
    while let Some(p) = n {
        if matches!(p.kind(), "function_definition" | "lambda" | "class_definition") {
            return true;
        }
        n = p.parent();
    }
    false
}

fn is_main_guard(if_node: Node<'_>, src: &[u8]) -> bool {
    let cond = if_node.child_by_field_name("condition").map(|c| text(c, src)).unwrap_or("");
    let c: String = cond.chars().filter(|c| !c.is_whitespace()).collect::<String>().replace('\'', "\"");
    c == "__name__==\"__main__\"" || c == "\"__main__\"==__name__"
}

fn in_main_guard(node: Node<'_>, src: &[u8]) -> bool {
    let mut n = node.parent();
    while let Some(p) = n {
        if p.kind() == "if_statement" && is_main_guard(p, src) {
            return true;
        }
        n = p.parent();
    }
    false
}

fn is_docstring(node: Node<'_>) -> bool {
    node.parent().is_some_and(|p| p.kind() == "expression_statement" && p.named_child_count() == 1)
}

/// The function part of a call as written: "open", "os.system", "subprocess.run".
fn callee(call: Node<'_>, src: &[u8]) -> String {
    call.child_by_field_name("function").map(|f| text(f, src).replace(char::is_whitespace, "")).unwrap_or_default()
}

/// Positional arguments of a call (keywords through `keyword`).
fn positional(call: Node<'_>) -> Vec<Node<'_>> {
    let mut out = Vec::new();
    if let Some(list) = call.child_by_field_name("arguments") {
        let mut cursor = list.walk();
        for a in list.named_children(&mut cursor) {
            if !matches!(a.kind(), "keyword_argument" | "comment" | "list_splat" | "dictionary_splat") {
                out.push(a);
            }
        }
    }
    out
}

fn keyword<'t>(call: Node<'t>, src: &[u8], name: &str) -> Option<Node<'t>> {
    let list = call.child_by_field_name("arguments")?;
    let mut cursor = list.walk();
    let found = list.named_children(&mut cursor).find(|a| {
        a.kind() == "keyword_argument" && a.child_by_field_name("name").is_some_and(|n| text(n, src) == name)
    });
    found.and_then(|a| a.child_by_field_name("value"))
}

/// Path checks on a string's value: Some((rule, targets…)) when it's one of ours.
pub fn absolute_path_targets(value: &str) -> Option<Vec<TargetOs>> {
    let v = value.trim();
    let bytes = v.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    let unc = v.starts_with("\\\\") && v.len() > 3 && v.as_bytes()[2].is_ascii_alphanumeric();
    if (drive && v.len() > 3) || unc {
        return Some(vec![TargetOs::Macos]);
    }
    if v.starts_with("/Users/") && v.len() > "/Users/".len() {
        return Some(vec![TargetOs::Windows]);
    }
    if v.starts_with("/home/") && v.len() > "/home/".len() {
        return Some(vec![TargetOs::Windows, TargetOs::Macos]);
    }
    None
}

/// A Unix system file: (where it exists, which targets lack it).
pub fn system_path(value: &str) -> Option<(&'static str, Vec<TargetOs>)> {
    let v = value.trim();
    if (v.starts_with("/proc/") || v.starts_with("/sys/")) && v.len() > 6 {
        return Some(("only exists on Linux", vec![TargetOs::Windows, TargetOs::Macos]));
    }
    if v.starts_with("/etc/") && v.len() > 5 && !v.contains(char::is_whitespace) {
        return Some(("doesn't exist on Windows", vec![TargetOs::Windows]));
    }
    None
}

/// A string that's a path piece being glued: "/", "\\", "/data", "out\\".
pub fn path_piece(value: &str) -> Option<char> {
    let v = value;
    if v.is_empty() || v.len() > 80 || v.contains(char::is_whitespace) || v.contains("://") || v.contains('%') {
        return None;
    }
    // Values arrive unescaped: a newline is a control character, not "\n".
    if v.starts_with("//") || v.chars().any(char::is_control) {
        return None;
    }
    let looks = |sep: &str| v.starts_with(sep) || v.ends_with(sep);
    if looks("\\") {
        return Some('\\');
    }
    if looks("/") && !v.chars().any(|c| matches!(c, '<' | '>' | '(' | ')' | '[' | ']' | '^' | '$' | '*' | '+' | '?')) {
        return Some('/');
    }
    None
}

fn path_concat_hit(rel: &str, node: Node<'_>, src: &[u8], sep: char) -> Hit {
    let rule = if sep == '\\' { "path-concat-backslash" } else { "path-concat-slash" };
    Hit::new(rule).at(rel, line(node)).evidence(evidence(node, src))
}

pub fn check(ctx: &Ctx<'_>, rel: &str, src: &[u8], tree: &tree_sitter::Tree) -> Vec<Hit> {
    let root = tree.root_node();
    let mut hits = Vec::new();
    let mut imports: HashSet<String> = HashSet::new();
    // Names imported from os / signal, so `from os import fork` is caught too.
    let mut from_os: HashSet<String> = HashSet::new();
    let mut from_signal: HashSet<String> = HashSet::new();
    let mut has_main_guard = false;

    // Imports first: they decide what later names mean.
    walk_nodes(root, |n| match n.kind() {
        "import_statement" => {
            let mut cursor = n.walk();
            for name in n.children_by_field_name("name", &mut cursor) {
                let module = match name.kind() {
                    "aliased_import" => {
                        name.child_by_field_name("name").map(|m| first_dotted(m, src)).unwrap_or_default()
                    }
                    _ => first_dotted(name, src),
                };
                imports.insert(module.clone());
                import_rules(ctx, rel, n, src, &module, &mut hits);
            }
        }
        "import_from_statement" => {
            if let Some(m) = n.child_by_field_name("module_name") {
                if m.kind() == "relative_import" {
                    return;
                }
                let module = first_dotted(m, src);
                imports.insert(module.clone());
                let full = text(m, src);
                let mut cursor = n.walk();
                let names: Vec<String> = n
                    .children_by_field_name("name", &mut cursor)
                    .map(|x| match x.kind() {
                        "aliased_import" => {
                            x.child_by_field_name("name").map(|y| text(y, src).to_owned()).unwrap_or_default()
                        }
                        _ => text(x, src).to_owned(),
                    })
                    .collect();
                if full == "os" {
                    from_os.extend(names.iter().cloned());
                    for name in &names {
                        if let Some((_, advice)) = UNIX_OS_API.iter().find(|(f, _)| f == name)
                            && !guarded(n, src)
                        {
                            hits.push(
                                Hit::new("unix-api")
                                    .at(rel, line(n))
                                    .evidence(evidence(n, src))
                                    .var("name", name)
                                    .var("alternative", *advice),
                            );
                        }
                    }
                } else if full == "signal" {
                    from_signal.extend(names.iter().cloned());
                }
                import_rules(ctx, rel, n, src, &module, &mut hits);
            }
        }
        "if_statement" if is_main_guard(n, src) => has_main_guard = true,
        _ => {}
    });

    let uses_processes =
        imports.contains("multiprocessing") || imports.contains("concurrent") || imports.contains("torch");
    let floor = ctx.snap.python_floor;
    let utf8_default = floor.is_some_and(|f| f >= (3, 15));
    // Top-level functions that start processes, and top-level calls to them.
    let mut pending_pool_hits: Vec<(String, Hit)> = Vec::new();

    walk_nodes(root, |n| {
        match n.kind() {
            "call" => {
                let f = callee(n, src);
                let last = f.rsplit('.').next().unwrap_or(&f).to_owned();

                // open() / read_text() / write_text() without encoding
                if !utf8_default {
                    let positional = positional(n);
                    let has_encoding = keyword(n, src, "encoding").is_some();
                    let binary = || {
                        let mode = keyword(n, src, "mode").or_else(|| positional.get(1).copied());
                        mode.and_then(|m| string_value(m, src)).is_some_and(|m| m.contains('b'))
                    };
                    let flagged = match f.as_str() {
                        "open" | "io.open" => !has_encoding && positional.len() < 4 && !binary(),
                        _ if last == "read_text" && f.contains('.') => !has_encoding && positional.is_empty(),
                        _ if last == "write_text" && f.contains('.') => !has_encoding && positional.len() < 2,
                        _ => false,
                    };
                    if flagged {
                        hits.push(Hit::new("open-encoding").at(rel, line(n)).evidence(evidence(n, src)));
                    }
                }

                // Process pools without a __main__ guard
                if uses_processes && matches!(last.as_str(), "Pool" | "Process" | "ProcessPoolExecutor") {
                    let hit = Hit::new("process-guard").at(rel, line(n)).evidence(evidence(n, src));
                    if !in_function(n) && !in_main_guard(n, src) {
                        hits.push(hit);
                    } else if let Some(name) = enclosing_top_function(n, src)
                        && !has_main_guard
                    {
                        pending_pool_hits.push((name, hit));
                    }
                }

                // Shell commands the other OS lacks
                if matches!(
                    f.as_str(),
                    "subprocess.run"
                        | "subprocess.call"
                        | "subprocess.check_call"
                        | "subprocess.check_output"
                        | "subprocess.Popen"
                        | "subprocess.getoutput"
                        | "subprocess.getstatusoutput"
                        | "os.system"
                        | "os.popen"
                ) && !guarded(n, src)
                {
                    let positional = positional(n);
                    if let Some(first) = positional.first() {
                        let word = match first.kind() {
                            "list" | "tuple" => first.named_child(0).and_then(|s| string_value(s, src)),
                            _ => {
                                string_value(*first, src).map(|s| s.split_whitespace().next().unwrap_or("").to_owned())
                            }
                        };
                        if let Some(word) = word {
                            let word = word.trim().trim_start_matches("./").to_ascii_lowercase();
                            if let Some((cmd, missing, advice)) = SHELL_COMMANDS.iter().find(|(c, _, _)| *c == word) {
                                hits.push(
                                    Hit::new("shell-command")
                                        .at(rel, line(n))
                                        .evidence(evidence(n, src))
                                        .only(*missing)
                                        .var("command", *cmd)
                                        .var("alternative", *advice),
                                );
                            }
                        }
                    }
                }

                // Environment variables via os.getenv / os.environ.get
                if matches!(f.as_str(), "os.getenv" | "getenv" | "os.environ.get" | "environ.get") {
                    let positional = positional(n);
                    let has_default = positional.len() >= 2 || keyword(n, src, "default").is_some();
                    if let Some(name) = positional.first().and_then(|a| string_value(*a, src))
                        && !has_default
                        && !guarded(n, src)
                    {
                        env_hit(rel, n, src, &name, &mut hits);
                    }
                }

                // CUDA: .cuda() calls, device="cuda", .to("cuda"), torch.device("cuda")
                let cuda_arg =
                    |a: Option<Node<'_>>| a.and_then(|a| string_value(a, src)).is_some_and(|v| v.starts_with("cuda"));
                let is_cuda = (last == "cuda" && f.contains('.') && !f.starts_with("torch.cuda"))
                    || cuda_arg(keyword(n, src, "device"))
                    || ((last == "to" || f == "torch.device") && cuda_arg(positional(n).first().copied()));
                if is_cuda && !guarded(n, src) {
                    hits.push(Hit::new("cuda").at(rel, line(n)).evidence(evidence(n, src)));
                }
            }
            // os.environ["HOME"]
            "subscript" => {
                let value = n.child_by_field_name("value").map(|v| text(v, src)).unwrap_or("");
                if matches!(value, "os.environ" | "environ")
                    && let Some(key) = n.child_by_field_name("subscript").and_then(|s| string_value(s, src))
                    && !guarded(n, src)
                {
                    env_hit(rel, n, src, &key, &mut hits);
                }
            }
            // os.fork, signal.SIGKILL, os.startfile …
            "attribute" => {
                let object = n.child_by_field_name("object").map(|o| text(o, src)).unwrap_or("");
                let attr = n.child_by_field_name("attribute").map(|a| text(a, src)).unwrap_or("");
                if guarded(n, src) {
                    return;
                }
                if object == "os"
                    && let Some((_, advice)) = UNIX_OS_API.iter().find(|(name, _)| *name == attr)
                {
                    hits.push(
                        Hit::new("unix-api")
                            .at(rel, line(n))
                            .evidence(evidence(n, src))
                            .var("name", attr)
                            .var("alternative", *advice),
                    );
                }
                if object == "signal" && UNIX_SIGNALS.contains(&attr) {
                    hits.push(Hit::new("unix-signal").at(rel, line(n)).evidence(evidence(n, src)).var("name", attr));
                }
                if let Some((_, _, advice)) = WINDOWS_API.iter().find(|(o, a, _)| *o == object && *a == attr) {
                    hits.push(
                        Hit::new("windows-api")
                            .at(rel, line(n))
                            .evidence(evidence(n, src))
                            .var("name", format!("{object}.{attr}"))
                            .var("alternative", *advice),
                    );
                }
            }
            // Bare names imported from os / signal
            "identifier" => {
                let name = text(n, src);
                let parent_kind = n.parent().map(|p| p.kind()).unwrap_or("");
                if parent_kind == "call"
                    && n.parent().and_then(|p| p.child_by_field_name("function")).is_some_and(|f| f.id() == n.id())
                    && from_signal.contains(name)
                    && UNIX_SIGNALS.contains(&name)
                    && !guarded(n, src)
                {
                    hits.push(Hit::new("unix-signal").at(rel, line(n)).evidence(evidence(n, src)).var("name", name));
                }
            }
            // Strings: hard-coded absolute paths, glued path pieces
            "string" if !is_docstring(n) => {
                let Some(value) = string_value(n, src) else { return };
                if let Some(targets) = absolute_path_targets(&value) {
                    let mut hit = Hit::new("absolute-path")
                        .at(rel, line(n))
                        .evidence(evidence(n, src))
                        .var("path", parse::shorten(&value, 60));
                    hit.targets = Some(targets);
                    hits.push(hit);
                    return;
                }
                if let Some((where_, targets)) = system_path(&value) {
                    let mut hit = Hit::new("system-path")
                        .at(rel, line(n))
                        .evidence(evidence(n, src))
                        .var("path", parse::shorten(&value, 60))
                        .var("where", where_);
                    hit.targets = Some(targets);
                    hits.push(hit);
                    return;
                }
                // f-strings that glue paths: f"{base}/{name}"
                let raw = text(n, src);
                if raw.starts_with(['f', 'F', 'r', 'R']) && raw.contains('{') {
                    let has = |sep: &str| value.contains(&format!("}}{sep}{{")) || value.contains(&format!("}}{sep}"));
                    if !value.contains("://") && !value.contains(char::is_whitespace) && !parse::builds_url(n, src) {
                        if has("\\\\") || has("\\") {
                            hits.push(path_concat_hit(rel, n, src, '\\'));
                        } else if has("/") && !value.contains('%') {
                            hits.push(path_concat_hit(rel, n, src, '/'));
                        }
                    }
                }
            }
            // base + "/" + name
            "binary_operator" => {
                let op = n.child_by_field_name("operator").map(|o| text(o, src)).unwrap_or("");
                if op != "+" {
                    return;
                }
                let (Some(left), Some(right)) = (n.child_by_field_name("left"), n.child_by_field_name("right")) else {
                    return;
                };
                if parse::builds_url(n, src) {
                    return;
                }
                // Only the innermost "+" with a string side, to report each glue once.
                for (lit, other) in [(left, right), (right, left)] {
                    if lit.kind() == "string"
                        && !text(lit, src).starts_with(['f', 'F'])
                        && other.kind() != "string"
                        && other.kind() != "binary_operator"
                        && let Some(sep) = string_value(lit, src).and_then(|v| path_piece(&v))
                    {
                        hits.push(path_concat_hit(rel, n, src, sep));
                        break;
                    }
                    if lit.kind() == "string"
                        && other.kind() == "binary_operator"
                        && let Some(sep) = string_value(lit, src).and_then(|v| path_piece(&v))
                    {
                        hits.push(path_concat_hit(rel, n, src, sep));
                        break;
                    }
                }
            }
            _ => {}
        }
    });

    // A top-level call to a function that starts processes, with no guard anywhere.
    if !has_main_guard && !pending_pool_hits.is_empty() {
        let mut called_at_top: HashSet<String> = HashSet::new();
        let mut cursor = root.walk();
        for stmt in root.named_children(&mut cursor) {
            if stmt.kind() == "expression_statement"
                && let Some(call) = stmt.named_child(0).filter(|c| c.kind() == "call")
            {
                called_at_top.insert(callee(call, src));
            }
        }
        for (name, hit) in pending_pool_hits {
            if called_at_top.contains(&name) {
                hits.push(hit);
            }
        }
    }
    dedupe(&mut hits);
    hits
}

fn enclosing_top_function(node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut n = node.parent();
    let mut last_fn = None;
    while let Some(p) = n {
        if p.kind() == "function_definition" {
            last_fn = p.child_by_field_name("name").map(|x| text(x, src).to_owned());
        }
        n = p.parent();
    }
    last_fn
}

fn env_hit(rel: &str, n: Node<'_>, src: &[u8], name: &str, hits: &mut Vec<Hit>) {
    if let Some((var, missing, advice)) = ENV_VARS.iter().find(|(v, _, _)| *v == name) {
        hits.push(
            Hit::new("env-var")
                .at(rel, line(n))
                .evidence(evidence(n, src))
                .only(*missing)
                .var("name", *var)
                .var("alternative", *advice),
        );
    }
}

fn import_rules(_ctx: &Ctx<'_>, rel: &str, n: Node<'_>, src: &[u8], module: &str, hits: &mut Vec<Hit>) {
    if guarded(n, src) {
        return;
    }
    let nested = in_function(n);
    let state = |top: State| if nested { State::WillNeedAFix } else { top };
    let base = || Hit::new("x").at(rel, line(n)).evidence(evidence(n, src));
    if let Some((m, alt)) = UNIX_MODULES.iter().find(|(m, _)| *m == module) {
        let mut h = base().var("module", *m).var("alternative", *alt).state(state(State::Blocker));
        h.rule = "unix-module";
        hits.push(h);
    }
    if let Some((m, alt)) = WINDOWS_MODULES.iter().find(|(m, _)| *m == module) {
        let mut h = base().var("module", *m).var("alternative", *alt).state(state(State::Blocker));
        h.rule = "windows-module";
        hits.push(h);
    }
    if MLX_MODULES.contains(&module) {
        let mut h = base().state(state(State::Blocker));
        h.rule = "mlx";
        hits.push(h);
    }
    if let Some((m, top)) = CUDA_LIBRARIES.iter().find(|(m, _)| *m == module) {
        let mut h = base().var("module", *m).state(state(*top));
        if *top == State::WillNeedAFix {
            h = h.severity(Severity::Medium);
        }
        h.rule = "cuda-only-library";
        hits.push(h);
    }
}

/// One hit per rule and line.
fn dedupe(hits: &mut Vec<Hit>) {
    let mut seen = HashSet::new();
    hits.retain(|h| seen.insert((h.rule, h.file.clone(), h.line)));
}

/// Code cells of a notebook, each with its cell number. Magics and shell
/// lines (`%…`, `!…`) are blanked so line numbers stay right.
pub fn notebook_cells(json: &str) -> Vec<(usize, String)> {
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(json) else { return vec![] };
    let mut out = Vec::new();
    for (i, cell) in doc.get("cells").and_then(|c| c.as_array()).into_iter().flatten().enumerate() {
        if cell.get("cell_type").and_then(|t| t.as_str()) != Some("code") {
            continue;
        }
        let source = match cell.get("source") {
            Some(serde_json::Value::Array(lines)) => lines.iter().filter_map(|l| l.as_str()).collect::<String>(),
            Some(serde_json::Value::String(s)) => s.clone(),
            _ => continue,
        };
        let cleaned: String = source
            .lines()
            .map(|l| if l.trim_start().starts_with(['%', '!']) { "" } else { l })
            .collect::<Vec<_>>()
            .join("\n");
        out.push((i + 1, cleaned));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_paths_are_told_apart_from_ordinary_strings() {
        assert_eq!(absolute_path_targets(r"C:\Users\ana\data.csv"), Some(vec![TargetOs::Macos]));
        assert_eq!(absolute_path_targets("/Users/ana/models"), Some(vec![TargetOs::Windows]));
        assert_eq!(absolute_path_targets("/home/ana/x"), Some(vec![TargetOs::Windows, TargetOs::Macos]));
        for fine in ["/api/users", "/tmp", "C:", "Users/ana", "/Users/", "https://x.io/Users/a"] {
            assert_eq!(absolute_path_targets(fine), None, "{fine}");
        }
    }

    #[test]
    fn platform_checks_are_whole_words_and_known_calls() {
        for yes in [
            "sys.platform == \"win32\"",
            "os.name == \"nt\"",
            "IS_WINDOWS",
            "platform.system() == \"Darwin\"",
            "hasattr(os, \"fork\")",
        ] {
            assert!(is_platform_check(yes), "{yes}");
        }
        for no in ["count > 0", "window_size > 3", "x is None", "options.verbose"] {
            assert!(!is_platform_check(no), "{no}");
        }
    }

    #[test]
    fn path_pieces_are_separators_at_the_edges() {
        assert_eq!(path_piece("/"), Some('/'));
        assert_eq!(path_piece("/data"), Some('/'));
        assert_eq!(path_piece("out\\"), Some('\\'));
        for fine in ["https://a.b/", "a b/", "%Y/%m", "1/2", "\n", "\r", "(/)", ""] {
            assert_eq!(path_piece(fine), None, "{fine}");
        }
    }
}
