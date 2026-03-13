/// Rewrite incoming path by stripping mount_path prefix and prefixing upstream_path.
/// Both mount_path and upstream_path are expected to start with '/'.
pub fn rewrite_path(incoming_path: &str, mount_path: &str, upstream_path: &str) -> String {
    let stripped = incoming_path
        .strip_prefix(mount_path)
        .unwrap_or(incoming_path);
    let normalized_upstream = normalize_leading(upstream_path);
    let stripped = strip_duplicate_upstream_prefix(stripped, &normalized_upstream);

    // ensure sub_path starts with '/' unless empty
    let sub_path_owned = if stripped.is_empty() {
        String::new()
    } else if stripped.starts_with('/') {
        stripped.to_string()
    } else {
        format!("/{stripped}")
    };

    let mut new_path = String::new();
    new_path.push_str(&normalized_upstream);
    new_path.push_str(&sub_path_owned);
    normalize_path(&new_path)
}

fn normalize_leading(p: &str) -> String {
    if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{}", p)
    }
}

/// Collapse duplicate slashes except after scheme boundary.
fn normalize_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    let mut prev_slash = false;
    for ch in p.chars() {
        if ch == '/' {
            if prev_slash {
                continue;
            }
            prev_slash = true;
        } else {
            prev_slash = false;
        }
        out.push(ch);
    }
    if out.is_empty() { "/".into() } else { out }
}

fn strip_duplicate_upstream_prefix<'a>(stripped: &'a str, upstream_path: &str) -> &'a str {
    if upstream_path == "/" {
        return stripped;
    }

    if stripped == upstream_path {
        ""
    } else if let Some(rest) = stripped.strip_prefix(upstream_path) {
        if rest.is_empty() || rest.starts_with('/') {
            rest
        } else {
            stripped
        }
    } else {
        stripped
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_basic() {
        let r = rewrite_path("/venue/api/v1", "/venue", "/api");
        assert_eq!(r, "/api/v1");
    }

    #[test]
    fn rewrite_root_mount() {
        let r = rewrite_path("/api/v1", "/", "/");
        assert_eq!(r, "/api/v1");
    }
}
