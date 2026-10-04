use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::State;
use tbb_core::models::BuildInvocation;
use tbb_core::preflight::{build_preflight, PreflightReport};
use tbb_core::system::{
    build_perl_bootstrap_args, check_rbm_perl_modules, cpanm_install_option, cpanm_path,
    perl_local_lib_root, platform_info, CapabilityStatus, CpanmInstallOption, PerlModuleCheck,
    PlatformInfo,
};

#[tauri::command]
pub async fn get_preflight_report(
    project_id: String,
    invocation: BuildInvocation,
    state: State<'_, AppState>,
) -> Result<PreflightReport, String> {
    let root = {
        let projects = state
            .projects
            .lock()
            .map_err(|_| "internal state lock poisoned".to_string())?;
        projects
            .get(&project_id)
            .map(|project| project.root_path.clone())
            .ok_or_else(|| "unknown project id".to_string())?
    };

    let shell_mode_allowed = state.settings.lock().await.safety.allow_shell_command_mode;
    Ok(build_preflight(&root, &invocation, shell_mode_allowed))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerlPreflightReport {
    pub platform: PlatformInfo,
    pub checks: Vec<PerlModuleCheck>,
    pub missing_modules: Vec<String>,
    pub blocking_modules: Vec<String>,
    pub cpanm_available: bool,
    pub cpanm_path: Option<String>,
    pub local_lib_root: Option<String>,
    pub install_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerlInstallResult {
    pub checks: Vec<PerlModuleCheck>,
    pub output: String,
    pub local_lib_root: Option<String>,
}

#[tauri::command]
pub async fn get_perl_preflight() -> Result<PerlPreflightReport, String> {
    Ok(build_perl_preflight())
}

fn build_perl_preflight() -> PerlPreflightReport {
    let checks = check_rbm_perl_modules();
    let missing_modules = checks
        .iter()
        .filter(|check| check.status == CapabilityStatus::Missing)
        .map(|check| check.module.clone())
        .collect::<Vec<_>>();
    let blocking_modules = checks
        .iter()
        .filter(|check| check.status != CapabilityStatus::Passed)
        .map(|check| check.module.clone())
        .collect::<Vec<_>>();

    let platform = platform_info();
    let cpanm = cpanm_path();
    let install_command = cpanm.as_ref().and_then(|_| {
        if missing_modules.is_empty() {
            None
        } else {
            let (_, args) = build_perl_bootstrap_args(&missing_modules, true);
            Some(format!("cpanm {}", shell_words(&args)))
        }
    });

    PerlPreflightReport {
        platform,
        checks,
        missing_modules,
        blocking_modules,
        cpanm_available: cpanm.is_some(),
        cpanm_path: cpanm,
        local_lib_root: perl_local_lib_root().map(|path| path.display().to_string()),
        install_command,
    }
}

#[tauri::command]
pub async fn install_cpanm(method: String) -> Result<String, String> {
    let option = cpanm_install_option(&method)
        .ok_or_else(|| format!("cpanm install method '{method}' is not available on this host"))?;

    if !option.can_run_in_app {
        return Err(format!(
            "TERMINAL_REQUIRED: {}\n{}",
            option.display_command, option.auth_note
        ));
    }

    #[cfg(target_os = "macos")]
    {
        if option.id == "macports" {
            let output = tbb_core::system::install_cpanm_with_macos_admin(&option)?;
            return verify_cpanm(&option, output);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = tbb_core::system::install_cpanm_with_pkexec(&option)?;
        return verify_cpanm(&option, output);
    }

    if option.id == "homebrew" {
        #[cfg(target_os = "macos")]
        {
            let brew = platform_info()
                .package_managers
                .into_iter()
                .find(|manager| manager.id == "homebrew")
                .ok_or_else(|| "Homebrew disappeared while installing cpanm".to_string())?;

            let output = Command::new(brew.path)
                .args(["install", "cpanminus"])
                .output()
                .map_err(|error| format!("could not start Homebrew: {error}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = combine_output(&stdout, &stderr);

            if !output.status.success() {
                return Err(format!(
                    "Homebrew cpanm installation failed (exit code {:?}).\n\n{}",
                    output.status.code(),
                    combined.trim()
                ));
            }

            return verify_cpanm(&option, combined);
        }
    }

    Err(format!(
        "No runnable installer implementation exists for '{}'.",
        option.id
    ))
}

fn verify_cpanm(option: &CpanmInstallOption, output: String) -> Result<String, String> {
    cpanm_path().map_or_else(
        || {
            Err(format!(
                "{} completed, but cpanm is still not visible to the app. Re-check PATH or restart the app.\n\n{}",
                option.label,
                output.trim()
            ))
        },
        |path| {
            Ok(format!(
                "{}\ncpanm: {}\n\n{}",
                option.label,
                path,
                output.trim()
            ))
        },
    )
}

#[tauri::command]
pub async fn install_rbm_perl_modules() -> Result<PerlInstallResult, String> {
    let before = build_perl_preflight();

    if before.blocking_modules.is_empty() {
        return Ok(PerlInstallResult {
            checks: before.checks,
            output: "All RBM Perl dependencies are already loadable.".to_string(),
            local_lib_root: before.local_lib_root,
        });
    }

    if before.cpanm_path.is_none() {
        return Err(format!(
            "cpanm is not installed. Choose one of the detected package-manager options first."
        ));
    }

    let cpanm = before
        .cpanm_path
        .ok_or_else(|| "cpanm disappeared while starting installation".to_string())?;

    let modules = before
        .blocking_modules
        .iter()
        .filter(|module| {
            before
                .checks
                .iter()
                .find(|check| &check.module == *module)
                .map(|check| check.status == CapabilityStatus::Missing)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();

    if modules.is_empty() {
        return Err(format!(
            "Perl reported non-passed dependency checks that are not safe to auto-install: {}",
            before.blocking_modules.join(", ")
        ));
    }

    let (_, args) = build_perl_bootstrap_args(&modules, true);
    let local_lib = perl_local_lib_root().ok_or_else(|| {
        "HOME is unavailable; cannot create the user-local Perl library.".to_string()
    })?;

    std::fs::create_dir_all(&local_lib)
        .map_err(|error| format!("could not create {}: {error}", local_lib.display()))?;

    let output = Command::new(&cpanm)
        .args(&args)
        .env("PERL_MM_USE_DEFAULT", "1")
        .output()
        .map_err(|error| format!("could not start cpanm: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = combine_output(&stdout, &stderr);

    if !output.status.success() {
        return Err(format!(
            "cpanm failed with exit code {:?}.\n\n{}",
            output.status.code(),
            combined.trim()
        ));
    }

    let after = build_perl_preflight();
    if !after.blocking_modules.is_empty() {
        return Err(format!(
            "cpanm finished, but these dependencies are still not ready: {}.\n\n{}",
            after.blocking_modules.join(", "),
            combined.trim()
        ));
    }

    Ok(PerlInstallResult {
        checks: after.checks,
        output: combined,
        local_lib_root: after.local_lib_root,
    })
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

fn shell_words(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "-_./:@+".contains(ch))
            {
                arg.clone()
            } else {
                format!("'{}'", arg.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
