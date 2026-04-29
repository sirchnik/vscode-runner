use std::collections::HashSet;

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
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let pattern = Pattern::new(query, CaseMatching::Ignore, Normalization::Smart, AtomKind::Fuzzy);

    // Identify which project names (last path component) match the query.
    let path_names: Vec<&str> = paths
        .iter()
        .map(|p| p.rsplit('/').next().unwrap_or(p.as_str()))
        .collect();
    let name_matched: HashSet<&str> = pattern
        .match_list(&path_names, &mut matcher)
        .into_iter()
        .map(|(name, _)| *name)
        .collect();

    // Match against full paths for scoring.
    let matching: Vec<(&str, u32)> = pattern
        .match_list(paths, &mut matcher)
        .into_iter()
        .map(|(s, score)| (s.as_str(), score))
        .collect();

    // Partition: paths whose project name matched come first,
    // preserving score order within each group.
    let (by_name, by_path): (Vec<_>, Vec<_>) = matching
        .into_iter()
        .partition::<Vec<_>, _>(|(path, _)| {
            let last = path.rsplit('/').next().unwrap_or(path);
            name_matched.contains(last)
        });

    by_name
        .into_iter()
        .map(|(path, score)| MatchedPath { path, score, is_name_match: true })
        .chain(
            by_path
                .into_iter()
                .map(|(path, score)| MatchedPath { path, score, is_name_match: false }),
        )
        .collect()
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
            "/home/user/master/master-thesis".to_string(),
            "/home/user/master/project-gamma".to_string(),
        ];
        let results = fuzzy_match_paths("master", &paths);

        let result_paths: Vec<&str> = results.iter().map(|r| r.path).collect();

        assert_eq!(result_paths.len(), 4);
        // Path with "master" in name comes first
        assert_eq!(result_paths[0], "/home/user/master/master-thesis");
        assert!(results[0].is_name_match);
        // Remaining are path-only matches
        assert!(!results[1].is_name_match);
        assert!(!results[2].is_name_match);
        assert!(!results[3].is_name_match);
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
