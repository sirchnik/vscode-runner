use nucleo_matcher::{
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
    Config, Matcher,
};

pub struct MatchedPath<'a> {
    pub path: &'a str,
    pub score: u32,
    pub is_name_match: bool,
}

/// Fuzzy-match `query` against `paths`, prioritizing matches on the last path component (project name).
///
/// Returns results ordered: name-matched paths first (sorted by score desc),
/// then remaining path-only matches (sorted by score desc).
pub fn fuzzy_match_paths<'a>(query: &str, paths: &'a [String]) -> Vec<MatchedPath<'a>> {
    log::debug!("Matching query '{query}' against paths:");
    for p in paths {
        log::debug!("  {p}");
    }
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let pattern = Pattern::new(
        query,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );
    let query_lower = query.to_ascii_lowercase();

    // Match against full paths for scoring.
    let matching: Vec<(&str, u32)> = pattern
        .match_list(paths, &mut matcher)
        .into_iter()
        .map(|(s, score)| (s.as_str(), score))
        .collect();

    let mut results: Vec<MatchedPath<'a>> = matching
        .into_iter()
        .map(|(path, score)| {
            let last = path.rsplit('/').next().unwrap_or(path);
            let is_name_match = last.to_ascii_lowercase().contains(&query_lower);

            MatchedPath {
                path,
                score,
                is_name_match,
            }
        })
        .collect();

    results.sort_by(|left, right| {
        right
            .is_name_match
            .cmp(&left.is_name_match)
            .then_with(|| right.score.cmp(&left.score))
    });

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finds_substring() {
        let paths = vec![
            "/home/user/projects/my-app".to_string(),
            "/home/user/projects/other".to_string(),
        ];
        let results = fuzzy_match_paths("my-app", &paths);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "/home/user/projects/my-app");
    }

    #[test]
    fn test_case_insensitive() {
        let paths = vec![
            "/home/user/MyProject".to_string(),
            "/home/user/other".to_string(),
        ];
        let results = fuzzy_match_paths("myproject", &paths);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "/home/user/MyProject");
    }

    #[test]
    fn test_partial_query() {
        let paths = vec![
            "/home/user/projects/vscode-runner".to_string(),
            "/home/user/projects/todo-app".to_string(),
            "/home/user/projects/web-server".to_string(),
        ];
        let results = fuzzy_match_paths("vscrun", &paths);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "/home/user/projects/vscode-runner");
    }

    #[test]
    fn test_no_results() {
        let paths = vec![
            "/home/user/projects/my-app".to_string(),
            "/home/user/projects/other".to_string(),
        ];
        let results = fuzzy_match_paths("zzzzz", &paths);

        assert!(results.is_empty());
    }

    #[test]
    fn test_scores_are_ordered() {
        let paths = vec![
            "/home/user/projects/foo-bar-baz".to_string(),
            "/home/user/projects/foobar".to_string(),
            "/home/user/projects/unrelated".to_string(),
        ];
        let results = fuzzy_match_paths("foobar", &paths);

        assert!(!results.is_empty());
        assert_eq!(results[0].path, "/home/user/projects/foobar");
    }

    #[test]
    fn test_prioritizes_last_path_component() {
        let paths = vec![
            "/home/user/master/project-alpha".to_string(),
            "/home/user/master/project-beta".to_string(),
            "/home/user/master/proj-examples/Class-B_Core_peripherals/mxb-example-ce2333486-safety-core-test.code-workspace".to_string(),
            "/home/user/master/master-thesis".to_string(),
            "/home/user/master/project-gamma".to_string(),
        ];
        let results = fuzzy_match_paths("master", &paths);

        let result_paths: Vec<&str> = results.iter().map(|r| r.path).collect();

        assert_eq!(result_paths.len(), 5);
        // Path with "master" in name comes first
        assert_eq!(result_paths[0], "/home/user/master/master-thesis");
    }

    #[test]
    fn test_exact_name_match_ranks_above_partial_path() {
        let paths = vec![
            "/home/user/myapp/server".to_string(),
            "/home/user/projects/myapp".to_string(),
        ];
        let results = fuzzy_match_paths("myapp", &paths);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, "/home/user/projects/myapp");
        assert!(results[0].is_name_match);
    }
}
