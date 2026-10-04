//! Real host capability checks plus explicit RBM Perl dependency preflight.
//! Detection is read-only. Installation is performed only by an explicit
//! preflight action.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Passed,
    Warning,
    Missing,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCheck {
    pub name: String,
    pub status: CapabilityStatus,
    pub detail: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostProfile {
    pub os: String,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub arch: String,
    pub cpu_cores: usize,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

fn run_version(exe: &str, args: &[&str]) -> ToolCheck {
    match Command::new(exe).args(args).output() {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let version = text.lines().next().map(|line| line.trim().to_string());
            ToolCheck {
                name: exe.to_string(),
                status: CapabilityStatus::Passed,
                detail: format!("found: {exe}"),
                version,
            }
        }
        Ok(out) => ToolCheck {
            name: exe.to_string(),
            status: CapabilityStatus::Warning,
            detail: format!(
                "'{exe} {}' exited with status {:?}",
                args.join(" "),
                out.status.code()
            ),
            version: None,
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ToolCheck {
            name: exe.to_string(),
            status: CapabilityStatus::Missing,
            detail: format!("'{exe}' not found"),
            version: None,
        },
        Err(error) => ToolCheck {
            name: exe.to_string(),
            status: CapabilityStatus::Unknown,
            detail: format!("could not run '{exe}': {error}"),
            version: None,
        },
    }
}

fn candidate_paths(exe: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(path_var) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path_var).map(|dir| dir.join(exe)));
    }

    if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home).join("perl5").join("bin").join(exe));
    }

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from(obfstr::obfstr!("/opt/homebrew/bin")).join(exe));
        paths.push(PathBuf::from(obfstr::obfstr!("/usr/local/bin")).join(exe));
        paths.push(PathBuf::from(obfstr::obfstr!("/opt/local/bin")).join(exe));
        paths.push(PathBuf::from(obfstr::obfstr!("/opt/local/sbin")).join(exe));
    }

    #[cfg(target_os = "linux")]
    {
        let linux_dirs = [
            obfstr::obfstr!("/usr/local/bin"),
            obfstr::obfstr!("/usr/bin"),
            obfstr::obfstr!("/bin"),
            obfstr::obfstr!("/usr/local/sbin"),
            obfstr::obfstr!("/usr/sbin"),
            obfstr::obfstr!("/sbin"),
        ];

        for dir in linux_dirs.iter().map(|d| d.as_str()) {
            paths.push(PathBuf::from(dir).join(exe));
        }
    }

    let mut unique = Vec::with_capacity(paths.len());
    for path in paths {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    unique
}

fn find_on_path(exe: &str) -> Option<String> {
    candidate_paths(exe)
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.display().to_string())
}

pub fn host_profile() -> HostProfile {
    let mut sys = System::new_all();
    sys.refresh_all();
    HostProfile {
        os: System::name().unwrap_or_else(|| "unknown".to_string()),
        os_version: System::os_version(),
        kernel_version: System::kernel_version(),
        arch: std::env::consts::ARCH.to_string(),
        cpu_cores: sys.cpus().len(),
        total_memory_bytes: sys.total_memory(),
        available_memory_bytes: sys.available_memory(),
        hostname: System::host_name(),
    }
}

pub fn disk_info_for(path: &Path) -> Option<DiskInfo> {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();
    let mut best: Option<&sysinfo::Disk> = None;
    let mut best_len = 0usize;

    for disk in disks.iter() {
        let mount = disk.mount_point();
        if path.starts_with(mount) {
            let len = mount.as_os_str().len();
            if len > best_len {
                best_len = len;
                best = Some(disk);
            }
        }
    }

    best.map(|disk| DiskInfo {
        mount_point: disk.mount_point().to_string_lossy().to_string(),
        total_bytes: disk.total_space(),
        available_bytes: disk.available_space(),
    })
}

pub fn run_tool_checks() -> Vec<ToolCheck> {
    let mut checks = vec![
        run_version("git", &["--version"]),
        run_version("make", &["--version"]),
        run_version("python3", &["--version"]),
        run_version("perl", &["--version"]),
        run_version("cpanm", &["--version"]),
    ];

    if cfg!(target_os = "linux") {
        checks.push(run_version("podman", &["--version"]));
        checks.push(run_version("docker", &["--version"]));
        checks.push(run_version("newuidmap", &["--version"]));
        checks.push(run_version("newgidmap", &["--version"]));
    } else {
        for name in ["podman", "docker", "newuidmap", "newgidmap"] {
            checks.push(ToolCheck {
                name: name.to_string(),
                status: CapabilityStatus::NotApplicable,
                detail: "Linux-only container/user-namespace requirement".to_string(),
                version: None,
            });
        }
    }

    checks
}

/// External modules directly imported by the RBM.pm shown by the user.
/// Perl core modules are deliberately excluded.
pub const RBM_PERL_MODULES: &[&str] = &[
    "DateTime",
    "Path::Tiny",
    "YAML::XS",
    "Template",
    "File::Copy::Recursive",
    "String::ShellQuote",
    "Sort::Versions",
    "Data::UUID",
    "Data::Dump",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerlModuleCheck {
    pub module: String,
    pub status: CapabilityStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagerInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpanmInstallOption {
    pub id: String,
    pub label: String,
    pub package_manager: String,
    pub display_command: String,
    pub can_run_in_app: bool,
    pub requires_admin: bool,
    pub auth_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub os_version: Option<String>,
    pub arch: String,
    pub perl_path: Option<String>,
    pub package_managers: Vec<PackageManagerInfo>,
    pub cpanm_install_options: Vec<CpanmInstallOption>,
}

pub fn perl_local_lib_root() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("perl5"))
}

pub fn perl_local_lib_path() -> Option<PathBuf> {
    perl_local_lib_root().map(|root| root.join("lib").join("perl5"))
}

fn perl_path() -> Option<String> {
    find_on_path("perl")
}

pub fn cpanm_path() -> Option<String> {
    find_on_path("cpanm")
}

fn perl_version_short() -> Option<String> {
    let perl = perl_path()?;
    let out = Command::new(&perl)
        .args(["-MConfig", "-e", "print $Config{version}"])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let mut parts = version.split('.');
    let major = parts.next()?;
    let minor = parts.next()?;

    if major.is_empty() || minor.is_empty() {
        None
    } else {
        Some(format!("{major}.{minor}"))
    }
}

fn package_manager(path_name: &str, id: &str, name: &str) -> Option<PackageManagerInfo> {
    let path = find_on_path(path_name)?;
    Some(PackageManagerInfo {
        id: id.to_string(),
        name: name.to_string(),
        path,
    })
}

fn is_command_available(path: &str) -> bool {
    Path::new(path).is_file()
}

fn shell_word(word: &str) -> String {
    if word
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "-_./:@+".contains(ch))
    {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

fn shell_command(executable: &str, args: &[&str]) -> String {
    std::iter::once(shell_word(executable))
        .chain(args.iter().map(|arg| shell_word(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn macports_cpanm_port() -> String {
    perl_version_short()
        .map(|version| format!("p{version}-app-cpanminus"))
        .unwrap_or_else(|| "p5.34-app-cpanminus".to_string())
}

pub fn platform_info() -> PlatformInfo {
    let host = host_profile();
    let perl = perl_path();
    let mut package_managers = Vec::new();
    let mut options = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if let Some(brew) = package_manager("brew", "homebrew", "Homebrew") {
            options.push(CpanmInstallOption {
                id: "homebrew".to_string(),
                label: "Install cpanm with Homebrew".to_string(),
                package_manager: brew.name.clone(),
                display_command: format!("{} install cpanminus", brew.path),
                can_run_in_app: true,
                requires_admin: false,
                auth_note: "Runs Homebrew as the current user. No sudo/password prompt is used."
                    .to_string(),
            });
            package_managers.push(brew);
        }

        if let Some(port) = package_manager("port", "macports", "MacPorts") {
            let port_name = macports_cpanm_port();
            options.push(CpanmInstallOption {
                id: "macports".to_string(),
                label: "Install cpanm with MacPorts".to_string(),
                package_manager: port.name.clone(),
                display_command: format!("sudo {} install {port_name}", port.path),
                can_run_in_app: is_command_available("/usr/bin/osascript"),
                requires_admin: true,
                auth_note: "Uses the native macOS administrator authorization dialog. The app does not collect or store the password."
                    .to_string(),
            });
            package_managers.push(port);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(apt) = package_manager("apt-get", "apt", "APT") {
            options.push(linux_cpanm_option(
                "apt",
                "Install cpanm with APT",
                &apt,
                &["install", "-y", "cpanminus"],
            ));
            package_managers.push(apt);
        }

        if let Some(dnf) = package_manager("dnf", "dnf", "DNF") {
            options.push(linux_cpanm_option(
                "dnf",
                "Install cpanm with DNF",
                &dnf,
                &["install", "-y", "perl-App-cpanminus"],
            ));
            package_managers.push(dnf);
        }

        if let Some(pacman) = package_manager("pacman", "pacman", "Pacman") {
            options.push(linux_cpanm_option(
                "pacman",
                "Install cpanm with Pacman",
                &pacman,
                &["-S", "--needed", "--noconfirm", "cpanminus"],
            ));
            package_managers.push(pacman);
        }

        if let Some(zypper) = package_manager("zypper", "zypper", "Zypper") {
            options.push(linux_cpanm_option(
                "zypper",
                "Install cpanm with Zypper",
                &zypper,
                &["install", "-y", "perl-App-cpanminus"],
            ));
            package_managers.push(zypper);
        }
    }

    PlatformInfo {
        os: host.os,
        os_version: host.os_version,
        arch: host.arch,
        perl_path: perl,
        package_managers,
        cpanm_install_options: options,
    }
}

#[cfg(target_os = "linux")]
fn linux_cpanm_option(
    id: &str,
    label: &str,
    manager: &PackageManagerInfo,
    args: &[&str],
) -> CpanmInstallOption {
    let sudo_command = shell_command(&manager.path, args);
    let display_command = format!("sudo {sudo_command}");
    let pkexec_available = find_on_path("pkexec").is_some();

    CpanmInstallOption {
        id: id.to_string(),
        label: label.to_string(),
        package_manager: manager.name.clone(),
        display_command: display_command.clone(),
        can_run_in_app: pkexec_available,
        requires_admin: true,
        auth_note: if pkexec_available {
            "Uses the system authorization dialog through polkit.".to_string()
        } else {
            format!("Run in a terminal so sudo can prompt for your password: {display_command}")
        },
    }
}

pub fn check_perl_module(module: &str) -> PerlModuleCheck {
    let perl = perl_path().unwrap_or_else(|| "perl".to_string());
    let mut command = Command::new(&perl);

    if let Some(lib) = perl_local_lib_path() {
        command.arg("-I").arg(lib);
    }

    match command
        .arg(format!("-M{module}"))
        .arg("-e")
        .arg("1")
        .output()
    {
        Ok(out) if out.status.success() => PerlModuleCheck {
            module: module.to_string(),
            status: CapabilityStatus::Passed,
            detail: format!("loadable via {perl}"),
        },
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let detail = stderr
                .lines()
                .next()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .unwrap_or("Perl could not load the module")
                .to_string();
            let not_found = stderr.contains("Can't locate");

            PerlModuleCheck {
                module: module.to_string(),
                status: if not_found {
                    CapabilityStatus::Missing
                } else {
                    CapabilityStatus::Warning
                },
                detail: if not_found {
                    format!("not found via {perl}: {detail}")
                } else {
                    format!("load failed via {perl}: {detail}")
                },
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => PerlModuleCheck {
            module: module.to_string(),
            status: CapabilityStatus::Unknown,
            detail: "Perl executable was not found".to_string(),
        },
        Err(error) => PerlModuleCheck {
            module: module.to_string(),
            status: CapabilityStatus::Unknown,
            detail: format!("could not run Perl: {error}"),
        },
    }
}

pub fn check_rbm_perl_modules() -> Vec<PerlModuleCheck> {
    RBM_PERL_MODULES
        .iter()
        .map(|module| check_perl_module(module))
        .collect()
}

pub fn build_perl_bootstrap_args(modules: &[String], use_cpanm: bool) -> (String, Vec<String>) {
    if use_cpanm {
        let local_lib = perl_local_lib_root()
            .unwrap_or_else(|| PathBuf::from("perl5"))
            .display()
            .to_string();

        let mut args = vec![
            "--local-lib-contained".to_string(),
            local_lib,
            "--notest".to_string(),
        ];
        args.extend(modules.iter().cloned());
        ("cpanm".to_string(), args)
    } else {
        let mut args = vec!["-T".to_string()];
        args.extend(modules.iter().cloned());
        ("cpan".to_string(), args)
    }
}

pub fn cpanm_install_option(id: &str) -> Option<CpanmInstallOption> {
    platform_info()
        .cpanm_install_options
        .into_iter()
        .find(|option| option.id == id)
}

#[cfg(target_os = "macos")]
pub fn install_cpanm_with_macos_admin(option: &CpanmInstallOption) -> Result<String, String> {
    let manager = platform_info()
        .package_managers
        .into_iter()
        .find(|manager| manager.name == option.package_manager)
        .ok_or_else(|| "selected package manager is no longer available".to_string())?;

    let package = macports_cpanm_port();
    let command = format!(
        "{} install {}",
        shell_word(&manager.path),
        shell_word(&package)
    );
    let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        escaped
    );

    let output = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output()
        .map_err(|error| format!("could not start macOS administrator authorization: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = combine_output(&stdout, &stderr);

    if !output.status.success() {
        return Err(format!(
            "MacPorts cpanm installation failed (exit code {:?}).\n\n{}",
            output.status.code(),
            combined.trim()
        ));
    }

    Ok(combined.trim().to_string())
}

#[cfg(target_os = "linux")]
pub fn install_cpanm_with_pkexec(option: &CpanmInstallOption) -> Result<String, String> {
    let manager = platform_info()
        .package_managers
        .into_iter()
        .find(|manager| manager.name == option.package_manager)
        .ok_or_else(|| "selected package manager is no longer available".to_string())?;

    let args: Vec<String> = match option.id.as_str() {
        "apt" => vec!["install".into(), "-y".into(), "cpanminus".into()],
        "dnf" => vec!["install".into(), "-y".into(), "perl-App-cpanminus".into()],
        "pacman" => vec![
            "-S".into(),
            "--needed".into(),
            "--noconfirm".into(),
            "cpanminus".into(),
        ],
        "zypper" => vec!["install".into(), "-y".into(), "perl-App-cpanminus".into()],
        _ => return Err("unsupported Linux package-manager option".to_string()),
    };

    let pkexec = find_on_path("pkexec").ok_or_else(|| {
        format!(
            "graphical administrator authorization is unavailable. Run this in a terminal:\n{}",
            option.display_command
        )
    })?;

    let output = Command::new(pkexec)
        .arg(&manager.path)
        .args(&args)
        .output()
        .map_err(|error| format!("could not start package-manager authorization: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = combine_output(&stdout, &stderr);

    if !output.status.success() {
        return Err(format!(
            "package-manager installation failed (exit code {:?}).\n\n{}",
            output.status.code(),
            combined.trim()
        ));
    }

    Ok(combined.trim().to_string())
}

fn combine_output(stdout: &str, stderr: &str) -> String {
    if stderr.trim().is_empty() {
        stdout.to_string()
    } else if stdout.trim().is_empty() {
        stderr.to_string()
    } else {
        format!("{stdout}\n{stderr}")
    }
}

pub fn overall_linux_build_readiness_note() -> &'static str {
    "Official Tor Browser full builds are generally aimed at a current Linux environment with container/user-namespace support. Exact requirements vary by checked-out upstream revision; consult the checkout's own documentation."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_profile_reports_nonzero_cores_and_memory() {
        let profile = host_profile();
        assert!(profile.cpu_cores >= 1);
        assert!(profile.total_memory_bytes > 0);
        assert!(!profile.arch.is_empty());
    }

    #[test]
    fn tool_check_detects_a_real_present_tool() {
        let checks = run_tool_checks();
        let git = checks.iter().find(|check| check.name == "git").unwrap();
        assert_eq!(git.status, CapabilityStatus::Passed);
        assert!(git.version.is_some());
    }

    #[test]
    fn tool_check_reports_missing_honestly() {
        let check = run_version("definitely-not-a-real-tool-xyz", &["--version"]);
        assert_eq!(check.status, CapabilityStatus::Missing);
    }

    #[test]
    fn disk_info_resolves_for_a_real_path() {
        let tmp = tempfile::tempdir().unwrap();
        let info = disk_info_for(tmp.path());
        assert!(info.is_some());
        assert!(info.unwrap().total_bytes > 0);
    }

    #[test]
    fn rbm_dependency_list_matches_external_imports() {
        assert_eq!(
            RBM_PERL_MODULES,
            &[
                "DateTime",
                "Path::Tiny",
                "YAML::XS",
                "Template",
                "File::Copy::Recursive",
                "String::ShellQuote",
                "Sort::Versions",
                "Data::UUID",
                "Data::Dump",
            ]
        );
    }
}
