pub const DEFAULT_DETAIL_LIMIT: usize = 512;

#[must_use]
pub fn sanitized_detail(raw: &str, limit: usize) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut end = limit.min(trimmed.len());
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    trimmed[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_characters_become_spaces_and_trim() {
        assert_eq!(
            sanitized_detail("\t ERROR:\r\nwrite failed \n", 512),
            "ERROR: write failed"
        );
    }

    #[test]
    fn canary_paths_survive_sanitization_but_are_capped() {
        let raw = r"S3CR3T_CANARY_TOKEN at C:\Users\someone\secrets.txt";
        let sanitized = sanitized_detail(raw, 30);
        assert!(sanitized.starts_with("S3CR3T_CANARY_TOKEN"));
        assert!(sanitized.chars().count() <= 30);
    }

    #[test]
    fn multi_byte_boundaries_are_respected() {
        let raw = "é".repeat(600);
        let sanitized = sanitized_detail(&raw, 10);

        assert_eq!(sanitized.chars().count(), 5);
        assert_eq!(sanitized.len(), 10);
    }

    #[test]
    fn empty_stays_empty() {
        assert_eq!(sanitized_detail("   ", 512), "");
    }

    #[test]
    fn mid_character_limits_walk_back_to_the_boundary() {
        let raw = "\u{e9}".repeat(600);
        let sanitized = sanitized_detail(&raw, 11);
        assert_eq!(sanitized.len(), 10);
    }
}

#[cfg(test)]
mod walk_back_tests {
    use super::*;

    #[test]
    fn mid_character_limits_walk_back_to_the_boundary() {
        let raw = "é".repeat(600);
        let sanitized = sanitized_detail(&raw, 11);
        assert_eq!(sanitized.len(), 10);
    }
}
