use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    pub line_num: u64,
    pub score: i64,
    pub indices: Vec<usize>,
    pub line_text: String,
}

pub struct FuzzySearch {
    matcher: SkimMatcherV2,
}

impl FuzzySearch {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Search `lines` against `query`, returns matches sorted by score descending.
    pub fn search<'a>(
        &self,
        lines: impl Iterator<Item = (u64, &'a str)>,
        query: &str,
        limit: usize,
    ) -> Vec<FuzzyMatch> {
        let mut results: Vec<FuzzyMatch> = lines
            .filter_map(|(line_num, text)| {
                self.matcher
                    .fuzzy_indices(text, query)
                    .map(|(score, indices)| FuzzyMatch {
                        line_num,
                        score,
                        indices,
                        line_text: text.to_owned(),
                    })
            })
            .collect();

        results.sort_by_key(|r| std::cmp::Reverse(r.score));
        results.truncate(limit);
        results
    }
}

impl Default for FuzzySearch {
    fn default() -> Self {
        Self::new()
    }
}
