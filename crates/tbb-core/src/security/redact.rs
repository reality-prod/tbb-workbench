//! Conservative secret redaction for previews, bundles and exported
//! metadata. Prefers over-redaction to leaking a token.

use crate::models::EnvironmentOverride;
use regex::Regex;
use std::sync::LazyLock;

const SECRET_KEY_PATTERNS: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "passphrase",
    "key",
    "auth",
    "credential",
    "apikey",
    "cookie",
    "session",
];

static URL_CREDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"://([^/:@\s]+):([^/@\s]+)@").unwrap());
static URL_USER_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"://([A-Za-z0-9_\-\.]{20,})@").unwrap());
static AUTH_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(authorization\s*[:=]\s*)(bearer|basic|token)?\s*[^\s'\x22]+").unwrap()
});
static KV_SECRET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)\b([A-Za-z0-9_\-]*(?:token|secret|password|passwd|passphrase|apikey|api_key|access_key)[A-Za-z0-9_\-]*)(\s*[=:]\s*)(?:"[^"]*"|'[^']*'|[^\s"']+)"#,
    )
    .unwrap()
});
static PRIVATE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?(-----END [A-Z ]*PRIVATE KEY-----|$)")
        .unwrap()
});
static GH_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(ghp_[A-Za-z0-9]{20,}|glpat-[A-Za-z0-9_\-]{15,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16})\b").unwrap()
});

pub fn key_looks_secret(key: &str) -> bool {
    let lower = key.to_lowercase();
    SECRET_KEY_PATTERNS.iter().any(|pat| lower.contains(pat))
}

pub fn redact_credential_urls(text: &str) -> String {
    let step = URL_CREDS.replace_all(text, "://[REDACTED]:[REDACTED]@");
    URL_USER_TOKEN
        .replace_all(&step, "://[REDACTED]@")
        .to_string()
}

pub fn redact_value(_value: &str) -> &'static str {
    "[REDACTED]"
}

/// Redacts free text (log tails, diagnostic bundles).
pub fn redact_text(text: &str) -> String {
    let t = redact_credential_urls(text);
    let t = PRIVATE_KEY.replace_all(&t, "[REDACTED PRIVATE KEY]");
    let t = AUTH_HEADER.replace_all(&t, "${1}[REDACTED]");
    let t = KV_SECRET.replace_all(&t, "${1}${2}[REDACTED]");
    GH_TOKEN.replace_all(&t, "[REDACTED]").to_string()
}

pub fn redact_env(env: &[EnvironmentOverride], reveal: bool) -> Vec<EnvironmentOverride> {
    env.iter()
        .map(|e| {
            let secret = e.likely_secret || key_looks_secret(&e.key);
            EnvironmentOverride {
                key: e.key.clone(),
                value: if secret && !reveal {
                    redact_value(&e.value).to_string()
                } else {
                    redact_credential_urls(&e.value)
                },
                likely_secret: secret,
            }
        })
        .collect()
}

/// Valid environment variable name for overrides (prevents `A=B` injection
/// through the key field).
pub fn valid_env_key(key: &str) -> bool {
    !key.is_empty()
        && !key.starts_with(|c: char| c.is_ascii_digit())
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_secret_key_names() {
        assert!(key_looks_secret("GITHUB_TOKEN"));
        assert!(key_looks_secret("db_password"));
        assert!(key_looks_secret("Authorization"));
        assert!(!key_looks_secret("BUILD_TARGET"));
        assert!(!key_looks_secret("PATH"));
    }

    #[test]
    fn redacts_credentials_embedded_in_urls() {
        let out = redact_credential_urls("cloning https://alice:hunter2@example.com/repo.git");
        assert!(!out.contains("hunter2"));
        assert!(!out.contains("alice"));
    }

    #[test]
    fn redacts_tokens_headers_and_keys() {
        let text = "Authorization: Bearer abc.def.ghi\nexport API_TOKEN=xyz123\npassword: 'p w'\n-----BEGIN RSA PRIVATE KEY-----\nMIIB\n-----END RSA PRIVATE KEY-----\nghp_abcdefghijklmnopqrstuvwxyz0123";
        let out = redact_text(text);
        for leaked in ["abc.def.ghi", "xyz123", "p w", "MIIB", "ghp_abcdefghij"] {
            assert!(!out.contains(leaked), "leaked {leaked}: {out}");
        }
        assert!(out.contains("[REDACTED"));
    }

    #[test]
    fn leaves_normal_text_alone() {
        let t = "make: Entering directory '/home/u/tbb'\nwarning: unused variable";
        assert_eq!(redact_text(t), t);
    }

    #[test]
    fn env_redaction_honours_reveal() {
        let env = vec![EnvironmentOverride {
            key: "MY_SECRET".into(),
            value: "v".into(),
            likely_secret: false,
        }];
        assert_eq!(redact_env(&env, false)[0].value, "[REDACTED]");
        assert_eq!(redact_env(&env, true)[0].value, "v");
    }

    #[test]
    fn env_key_validation() {
        assert!(valid_env_key("MAKEFLAGS"));
        assert!(!valid_env_key("A=B"));
        assert!(!valid_env_key("1A"));
        assert!(!valid_env_key(""));
    }
}
