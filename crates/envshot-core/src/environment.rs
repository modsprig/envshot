use std::collections::BTreeSet;

pub const DEFAULT_VALUE_LIMIT: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentPolicy {
    allow_keys: BTreeSet<String>,
    value_limit: usize,
}

impl Default for EnvironmentPolicy {
    fn default() -> Self {
        Self::none()
    }
}

impl EnvironmentPolicy {
    #[must_use]
    pub fn none() -> Self {
        Self {
            allow_keys: BTreeSet::new(),
            value_limit: DEFAULT_VALUE_LIMIT,
        }
    }

    #[must_use]
    pub fn with_allow_key(mut self, key: impl Into<String>) -> Self {
        self.allow_keys.insert(key.into());
        self
    }

    #[must_use]
    pub fn with_value_limit(mut self, limit: usize) -> Self {
        self.value_limit = limit;
        self
    }

    #[must_use]
    pub fn allows(&self, key: &str) -> bool {
        self.allow_keys.contains(key) && !is_secret_key(key)
    }

    #[must_use]
    pub fn value_limit(&self) -> usize {
        self.value_limit
    }
}

const SECRET_FRAGMENTS: [&str; 8] = [
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "CREDENTIAL",
    "AUTHORIZATION",
    "PRIVATE_KEY",
    "SESSION",
];

#[must_use]
pub fn is_secret_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    if SECRET_FRAGMENTS
        .iter()
        .any(|fragment| upper.contains(fragment))
    {
        return true;
    }
    upper == "KEY" || upper.ends_with("_KEY") || upper.ends_with("PASS")
}

#[must_use]
pub fn bounded_value(value: &str, limit: usize) -> Option<&str> {
    (value.len() <= limit).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_collects_nothing() {
        let policy = EnvironmentPolicy::none();
        assert!(!policy.allows("PATH"));
        assert!(!policy.allows("RUST_VERSION"));
    }

    #[test]
    fn allowlisted_non_secret_keys_are_allowed() {
        let policy = EnvironmentPolicy::none().with_allow_key("RUST_VERSION");
        assert!(policy.allows("RUST_VERSION"));
    }

    #[test]
    fn secret_keys_are_never_allowed_even_when_allowlisted() {
        let policy = EnvironmentPolicy::none()
            .with_allow_key("API_TOKEN")
            .with_allow_key("DB_PASSWORD")
            .with_allow_key("AWS_SECRET_KEY")
            .with_allow_key("GPG_PRIVATE_KEY")
            .with_allow_key("GITHUB_TOKEN")
            .with_allow_key("SESSION_ID")
            .with_allow_key("VAULT_PASS")
            .with_allow_key("STRIPE_KEY");
        for key in [
            "API_TOKEN",
            "DB_PASSWORD",
            "AWS_SECRET_KEY",
            "GPG_PRIVATE_KEY",
            "GITHUB_TOKEN",
            "SESSION_ID",
            "VAULT_PASS",
            "STRIPE_KEY",
        ] {
            assert!(!policy.allows(key), "{key} must classify as a secret");
        }
    }

    #[test]
    fn values_over_the_limit_are_rejected() {
        let value = "x".repeat(64);
        assert_eq!(bounded_value(&value, 64), Some(value.as_str()));
        assert_eq!(bounded_value(&value, 63), None);
    }

    #[test]
    fn canary_secret_key_is_rejected() {
        let policy = EnvironmentPolicy::none().with_allow_key("S3CR3T_CANARY_TOKEN_SECRET");
        assert!(!policy.allows("S3CR3T_CANARY_TOKEN_SECRET"));
    }
}

#[cfg(test)]
mod limit_tests {
    use super::*;

    #[test]
    fn value_limit_is_overridable_and_reported() {
        let policy = EnvironmentPolicy::none().with_value_limit(64);
        assert_eq!(policy.value_limit(), 64);
        assert_eq!(EnvironmentPolicy::none().value_limit(), DEFAULT_VALUE_LIMIT);
    }
}
