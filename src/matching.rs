use krunner::{Match, MatchIcon, MatchType};
use nucleo_matcher::{
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};

#[derive(Debug, Clone, Copy)]
pub struct MatchedPath<'a> {
    pub path: &'a str,
    pub score: u32,
    pub is_name_match: bool,
    pub original_index: usize,
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
    // Nucleo's match_list returns (item, score). We need to incorporate the original index for recency.
    let mut results: Vec<MatchedPath<'a>> = Vec::new();
    let mut char_buf = Vec::new();

    for (i, path) in paths.iter().enumerate() {
        if let Some(score) = pattern.indices(
            Utf32Str::new(path, &mut char_buf),
            &mut matcher,
            &mut Vec::new(),
        ) {
            let last = path.rsplit('/').next().unwrap_or(path);
            let is_name_match = last.to_ascii_lowercase().contains(&query_lower);

            // Further boost score if the name match is closer to the original query length (less extra noise)
            let mut final_score = score;
            if is_name_match {
                final_score += 1000 / (1 + (last.len() as u32).saturating_sub(query.len() as u32));
            }

            results.push(MatchedPath {
                path,
                score: final_score,
                is_name_match,
                original_index: i,
            });
        }
    }

    results.sort_by(|left, right| {
        left.original_index
            .cmp(&right.original_index) // Lower index (more recent) comes first - PRIMARY FACTOR
            .then_with(|| right.is_name_match.cmp(&left.is_name_match)) // Then name matches
            .then_with(|| right.score.cmp(&left.score)) // Then score
    });

    results
}

pub fn match_relevance(m: &MatchedPath<'_>, max_score: f64) -> f64 {
    let recency_component = 1_000_000.0 / (m.original_index as f64 + 1.0);
    let score_component = if max_score > 0.0 {
        m.score as f64 / max_score
    } else {
        1.0
    };

    recency_component + if m.is_name_match { 100.0 } else { 0.0 } + score_component
}

pub fn build_krunner_matches<A>(
    query: &str,
    recent_paths: &[String],
    icon_name: &str,
    id_prefix: &str,
) -> Vec<Match<A>>
where
    A: krunner::Action,
{
    let results = fuzzy_match_paths(query, recent_paths);

    if results.is_empty() {
        return vec![];
    }

    let max_score = results[0].score as f64;
    let home_dir = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    results
        .into_iter()
        .map(|m| {
            let uri = path_to_uri(m.path);
            let project_name = m.path.rsplit('/').next().unwrap_or(m.path);
            let subtitle = parse_relative_path(&uri, &home_dir).unwrap_or(uri);
            let id = format!("{id_prefix}-{}", m.path);
            let relevance = match_relevance(&m, max_score);

            Match {
                id,
                title: project_name.to_owned(),
                icon: MatchIcon::ByName(icon_name.to_owned()),
                subtitle: Some(subtitle),
                ty: MatchType::ExactMatch,
                relevance,
                ..Match::default()
            }
        })
        .collect()
}

pub fn path_to_uri(path: &str) -> String {
    path.replace("file://", "").replace("vscode-remote://", "")
}

pub fn parse_relative_path(uri: &str, home_dir: &str) -> Option<String> {
    if !home_dir.is_empty() && uri.contains(home_dir) {
        let without_home = &uri[home_dir.len()..];
        Some(format!("~{without_home}"))
    } else {
        None
    }
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
            "/home/user/projects/foobar".to_string(), // index 0: better match, most recent
            "/home/user/projects/foo-bar-baz".to_string(), // index 1: lower score match
            "/home/user/projects/unrelated".to_string(), // index 2: no match
        ];
        let results = fuzzy_match_paths("foobar", &paths);

        assert!(!results.is_empty());
        // Most recent match should be first
        assert_eq!(results[0].path, "/home/user/projects/foobar");
    }

    #[test]
    fn test_prioritizes_last_path_component() {
        let paths = vec![
            "/home/user/master/master-thesis".to_string(),  // index 0: name match "master", most recent
            "/home/user/master/project-alpha".to_string(),
            "/home/user/master/project-beta".to_string(),
            "/home/user/master/proj-examples/Class-B_Core_peripherals/mxb-example-ce2333486-safety-core-test.code-workspace".to_string(),
            "/home/user/master/project-gamma".to_string(),
        ];
        let results = fuzzy_match_paths("master", &paths);

        let result_paths: Vec<&str> = results.iter().map(|r| r.path).collect();

        assert_eq!(result_paths.len(), 5);
        // Path with "master" in name should come first (index 0, most recent)
        assert_eq!(result_paths[0], "/home/user/master/master-thesis");
    }

    #[test]
    fn test_prioritize_whole_name_matches() {
        let paths = vec![
            "/home/user/master/tock1".to_string(), // index 0: whole name match, most recent
            "/home/user/master/tockloader".to_string(), // index 1: partial match, older
        ];
        let results = fuzzy_match_paths("tock", &paths);

        let result_paths: Vec<&str> = results.iter().map(|r| r.path).collect();

        assert_eq!(result_paths.len(), 2);
        // Most recent path comes first
        assert_eq!(result_paths[0], "/home/user/master/tock1");
    }

    #[test]
    fn test_exact_name_match_ranks_above_partial_path() {
        let paths = vec![
            "/home/user/projects/myapp".to_string(), // index 0: exact name match, most recent
            "/home/user/myapp/server".to_string(),   // index 1: partial match, older
        ];
        let results = fuzzy_match_paths("myapp", &paths);

        assert_eq!(results.len(), 2);
        // Most recent should be first
        assert_eq!(results[0].path, "/home/user/projects/myapp");
        assert!(results[0].is_name_match);
    }

    #[test]
    fn test_breaks_tie_with_original_order() {
        let paths = vec![
            "/home/user/recent-b".to_string(),
            "/home/user/recent-a".to_string(),
        ];
        // Both match "recent" equally in score and both are name matches.
        // The one that appears earlier in the input (index 0) should come first.
        let results = fuzzy_match_paths("recent", &paths);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, "/home/user/recent-b");
        assert_eq!(results[1].path, "/home/user/recent-a");
    }

    #[test]
    fn test_recent_projects_rank_higher_despite_lower_score() {
        // A recent project (index 0) should rank higher than an older project (index 5)
        // even if the older one has a higher fuzzy match score on the full path.
        let paths = vec![
            "/home/user/vscode-runner".to_string(), // index 0: name match, but may have lower overall score
            "/home/user/archived/old-vscrunner-backup".to_string(), // index 1: higher score but older
            "/home/user/old-projects/vscode-run-test".to_string(),  // index 2
            "/home/user/very-old/vscode".to_string(),               // index 3
            "/home/user/ancient/vscrun".to_string(),                // index 4
            "/home/user/prehistoric/v-s-c-run".to_string(),         // index 5: lowest recency
        ];

        let results = fuzzy_match_paths("vscrun", &paths);

        // The most recent project should be first despite potential score differences
        assert!(!results.is_empty());
        assert_eq!(results[0].path, "/home/user/vscode-runner");
    }

    #[test]
    fn test_recent_name_match_beats_older_name_match() {
        // When both are name matches, the more recent one should rank first
        let paths = vec![
            "/home/user/projects/app".to_string(), // index 0: name match, recent
            "/old/backups/projects/app".to_string(), // index 1: name match, older
            "/ancient/archive/projects/app".to_string(), // index 2: name match, very old
        ];

        let results = fuzzy_match_paths("app", &paths);

        assert_eq!(results.len(), 3);
        // Most recent should be first
        assert_eq!(results[0].path, "/home/user/projects/app");
        assert_eq!(results[1].path, "/old/backups/projects/app");
        assert_eq!(results[2].path, "/ancient/archive/projects/app");
    }

    #[test]
    fn test_recency_outweighs_small_score_difference() {
        // Recent projects should rank higher even with somewhat lower match quality
        let paths = vec![
            "/home/user/myproject".to_string(),           // index 0: recent
            "/archive/myproject-old-version".to_string(), // index 1: older, possibly higher score due to longer match
            "/backup/myproject-backup".to_string(),       // index 2: even older
        ];

        let results = fuzzy_match_paths("myproject", &paths);

        assert!(!results.is_empty());
        // Most recent should rank first
        assert_eq!(results[0].path, "/home/user/myproject");
    }

    #[test]
    fn test_master_query_with_real_data() {
        // Test case based on real data: when querying "master",
        // trusted-firmware-m should rank before zephyr/zephyr because it appears earlier in the list
        let paths = vec![
            "file:///home/user/Projects/master/tock1".to_string(), // index 0
            "file:///home/user/Projects/contrib/vscode-runner-1".to_string(), // index 1
            "file:///home/user/Projects/bachelor/bachelor".to_string(), // index 2
            "file:///home/user/Projects/piconut/ccleste".to_string(), // index 3
            "file:///home/user/Projects/master/trusted-firmware-m".to_string(), // index 4 - should be first among matches
            "file:///home/user/Projects/master/trustzone-m-rs".to_string(),     // index 5
            "file:///home/user/Projects/master/master-thesis".to_string(),      // index 6
            "file:///home/user/Projects/master/psoc-examples".to_string(),      // index 7
            "file:///home/user/Projects/master/zephyr/zephyr".to_string(), // index 8 - should be second
        ];

        let results = fuzzy_match_paths("master", &paths);

        // Both paths should match "master"
        let result_paths: Vec<&str> = results.iter().map(|r| r.path).collect();

        // Find indices of tf-m and zephyr in results
        let tfm_idx = result_paths
            .iter()
            .position(|p| p.contains("trusted-firmware-m"));
        let zephyr_idx = result_paths
            .iter()
            .position(|p| p.contains("zephyr/zephyr"));

        // Both should exist
        assert!(tfm_idx.is_some(), "trusted-firmware-m not found in results");
        assert!(zephyr_idx.is_some(), "zephyr/zephyr not found in results");

        // tf-m should come before zephyr (lower original index = more recent)
        assert!(
            tfm_idx < zephyr_idx,
            "trusted-firmware-m (index {:?}) should come before zephyr (index {:?})",
            tfm_idx,
            zephyr_idx
        );
    }

    #[test]
    fn test_match_relevance_prioritizes_recency_over_score() {
        let max_score = 100.0;
        let trusted_firmware = MatchedPath {
            path: "file:///home/user/Projects/master/trusted-firmware-m",
            score: 1,
            is_name_match: true,
            original_index: 4,
        };
        let zephyr = MatchedPath {
            path: "file:///home/user/Projects/master/zephyr/zephyr",
            score: 100,
            is_name_match: true,
            original_index: 8,
        };

        assert!(
            match_relevance(&trusted_firmware, max_score) > match_relevance(&zephyr, max_score)
        );
    }

    #[test]
    fn test_path_to_uri_strips_file_scheme() {
        assert_eq!(
            path_to_uri("file:///home/user/project"),
            "/home/user/project"
        );
    }

    #[test]
    fn test_path_to_uri_strips_vscode_remote_scheme() {
        assert_eq!(
            path_to_uri("vscode-remote:///home/user/project"),
            "/home/user/project"
        );
    }

    #[test]
    fn test_path_to_uri_plain_path_unchanged() {
        assert_eq!(path_to_uri("/home/user/project"), "/home/user/project");
    }

    #[test]
    fn test_parse_relative_path_with_home_dir() {
        let result = parse_relative_path("/home/user/projects/foo", "/home/user");
        assert_eq!(result, Some("~/projects/foo".to_owned()));
    }

    #[test]
    fn test_parse_relative_path_without_home_dir() {
        let result = parse_relative_path("/opt/projects/foo", "/home/user");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_relative_path_empty_home_dir() {
        let result = parse_relative_path("/home/user/projects/foo", "");
        assert_eq!(result, None);
    }
}
