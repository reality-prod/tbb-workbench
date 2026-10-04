use tbb_core::git::{
    build_checkout_invocation, build_clone_invocation, build_diff_patch_invocation,
    build_fetch_invocation, build_submodule_update_invocation, validate_destination, CloneRequest,
};
use tbb_core::models::BuildInvocation;

#[tauri::command]
pub fn validate_clone_destination(destination: String) -> Result<(), String> {
    validate_destination(std::path::Path::new(&destination))
}

#[tauri::command]
pub fn preview_clone_invocation(request: CloneRequest) -> BuildInvocation {
    build_clone_invocation(&request)
}

#[tauri::command]
pub fn preview_checkout_invocation(repo_root: String, git_ref: String) -> BuildInvocation {
    build_checkout_invocation(std::path::Path::new(&repo_root), &git_ref)
}

#[tauri::command]
pub fn preview_fetch_invocation(repo_root: String, remote: String) -> BuildInvocation {
    build_fetch_invocation(std::path::Path::new(&repo_root), &remote)
}

#[tauri::command]
pub fn preview_submodule_update_invocation(repo_root: String) -> BuildInvocation {
    build_submodule_update_invocation(std::path::Path::new(&repo_root))
}

#[tauri::command]
pub fn preview_diff_patch_invocation(repo_root: String, staged_too: bool) -> BuildInvocation {
    build_diff_patch_invocation(std::path::Path::new(&repo_root), staged_too)
}
