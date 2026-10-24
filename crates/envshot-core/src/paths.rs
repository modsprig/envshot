#[must_use]
pub fn symbolic_path(path: &str, home: Option<&str>) -> String {
    let Some(home) = home else {
        return path.to_owned();
    };
    if home.is_empty() || path.is_empty() {
        return path.to_owned();
    }

    let windows_style = home.as_bytes().get(1) == Some(&b':');

    let (path_normalized, home_normalized) = if windows_style {
        (path.to_lowercase(), home.to_lowercase())
    } else {
        (path.to_owned(), home.to_owned())
    };
    let path_normalized = normalize_separators(&path_normalized, windows_style);
    let home_normalized = normalize_separators(&home_normalized, windows_style);

    let boundary_matches = path_normalized == home_normalized
        || (path_normalized.starts_with(&home_normalized)
            && matches!(
                path_normalized.as_bytes().get(home_normalized.len()),
                Some(b'\\' | b'/')
            ));
    if !boundary_matches {
        return path.to_owned();
    }

    if !path.is_char_boundary(home.len()) {
        return path.to_owned();
    }
    let after_home = &path[home.len()..];
    let remainder = after_home.trim_start_matches(['\\', '/']);
    let token = if windows_style {
        "%USERPROFILE%"
    } else {
        "$HOME"
    };
    if remainder.is_empty() {
        token.to_owned()
    } else {
        let separator = &after_home[..after_home.len() - remainder.len()];
        format!("{token}{separator}{remainder}")
    }
}

fn normalize_separators(value: &str, windows_style: bool) -> String {
    if windows_style {
        value.replace('/', "\\")
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_home_becomes_userprofile_token() {
        assert_eq!(
            symbolic_path(r"C:\Users\Alice\project\file.txt", Some(r"C:\Users\Alice")),
            r"%USERPROFILE%\project\file.txt"
        );
    }

    #[test]
    fn windows_matching_is_case_and_separator_insensitive() {
        assert_eq!(
            symbolic_path("C:/Users/Alice/project", Some(r"c:\users\alice")),
            r"%USERPROFILE%/project"
        );
    }

    #[test]
    fn unix_home_becomes_dollar_home() {
        assert_eq!(
            symbolic_path("/home/alice/project", Some("/home/alice")),
            "$HOME/project"
        );
        assert_eq!(
            symbolic_path("/Users/alice/project", Some("/Users/alice")),
            "$HOME/project"
        );
    }

    #[test]
    fn bare_home_becomes_the_bare_token() {
        assert_eq!(
            symbolic_path(r"C:\Users\Alice", Some(r"C:\Users\Alice")),
            "%USERPROFILE%"
        );
        assert_eq!(symbolic_path("/home/alice", Some("/home/alice")), "$HOME");
    }

    #[test]
    fn single_character_remainder_keeps_its_separator() {
        assert_eq!(
            symbolic_path(r"C:\Users\Alice\x", Some(r"C:\Users\Alice")),
            r"%USERPROFILE%\x"
        );
    }

    #[test]
    fn component_boundaries_are_respected() {
        assert_eq!(
            symbolic_path("/home/alice2/project", Some("/home/alice")),
            "/home/alice2/project"
        );
    }

    #[test]
    fn non_home_paths_pass_through() {
        assert_eq!(
            symbolic_path(r"C:\Windows\system32\drivers", Some(r"C:\Users\Alice")),
            r"C:\Windows\system32\drivers"
        );
        assert_eq!(symbolic_path("/usr/lib", Some("/home/alice")), "/usr/lib");
    }

    #[test]
    fn unknown_home_never_guesses() {
        assert_eq!(
            symbolic_path("/home/alice/project", None),
            "/home/alice/project"
        );
    }

    #[test]
    fn empty_inputs_pass_through() {
        assert_eq!(symbolic_path("", Some("/home/alice")), "");
        assert_eq!(symbolic_path("/x", Some("")), "/x");
    }
}
