//! Renders argv as a string that can be pasted into a POSIX shell and
//! reproduces the same argv. Used for every command shown in the GUI.

use crate::models::EnvironmentOverride;
use crate::security::redact::redact_value;

pub fn shell_quote(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }
    let safe = arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:=@%+,".contains(c));
    if safe {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

pub fn display_command(exe: &str, args: &[String]) -> String {
    let mut parts = vec![shell_quote(exe)];
    parts.extend(args.iter().map(|a| shell_quote(a)));
    parts.join(" ")
}

/// Full reproducible line including env prefix and `cd`. Secret values are
/// replaced with a placeholder unless `reveal` is true.
pub fn display_full(
    cwd: &str,
    env: &[EnvironmentOverride],
    exe: &str,
    args: &[String],
    reveal: bool,
) -> String {
    let mut s = format!("cd {} && ", shell_quote(cwd));
    for e in env {
        let value = if e.likely_secret && !reveal {
            redact_value(&e.value).to_string()
        } else {
            e.value.clone()
        };
        s.push_str(&format!("{}={} ", e.key, shell_quote(&value)));
    }
    s.push_str(&display_command(exe, args));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_when_needed() {
        assert_eq!(shell_quote("make"), "make");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn display_is_reproducible() {
        let s = display_command("make", &["-j4".into(), "my target".into()]);
        assert_eq!(s, "make -j4 'my target'");
    }

    #[test]
    fn secrets_hidden_in_full_display() {
        let env = vec![EnvironmentOverride {
            key: "API_TOKEN".into(),
            value: "s3cret".into(),
            likely_secret: true,
        }];
        let s = display_full("/tmp/x", &env, "make", &[], false);
        assert!(!s.contains("s3cret"));
        let s2 = display_full("/tmp/x", &env, "make", &[], true);
        assert!(s2.contains("s3cret"));
    }
}
