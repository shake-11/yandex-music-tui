//! the genre tree.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Icon;

/// a genre's localized title.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GenreTitle {
    /// short title.
    pub title: Option<String>,
    /// full title.
    pub full_title: Option<String>,
}

/// a music genre, possibly containing sub-genres.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Genre {
    /// genre id.
    pub id: Option<String>,
    /// sorting weight.
    pub weight: Option<i64>,
    /// whether composers are highlighted for this genre.
    pub composer_top: Option<bool>,
    /// default title.
    pub title: Option<String>,
    /// localized titles keyed by language code.
    pub titles: HashMap<String, GenreTitle>,
    /// genre images, still free-form upstream.
    pub images: Option<Value>,
    /// whether shown in the genre menu.
    pub show_in_menu: Option<bool>,
    /// region codes where shown.
    pub show_in_regions: Vec<i64>,
    /// region codes where hidden.
    pub hide_in_regions: Vec<i64>,
    /// full title.
    pub full_title: Option<String>,
    /// url slug.
    pub url_part: Option<String>,
    /// accent color.
    pub color: Option<String>,
    /// radio icon.
    pub radio_icon: Option<Icon>,
    /// nested sub-genres.
    pub sub_genres: Vec<Genre>,
}

impl Genre {
    /// the localized title for a language code, falling back to the default.
    #[must_use]
    pub fn localized_title(&self, language: &str) -> Option<&str> {
        self.titles
            .get(language)
            .and_then(|t| t.title.as_deref())
            .or(self.title.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localized_titles_fall_back_to_the_default() {
        let genre: Genre =
            serde_json::from_str(r#"{"id":"rock","title":"Rock","titles":{"ru":{"title":"Рок"}}}"#)
                .unwrap();
        assert_eq!(genre.localized_title("ru"), Some("Рок"));
        assert_eq!(genre.localized_title("de"), Some("Rock"));
        assert_eq!(Genre::default().localized_title("ru"), None);
    }

    #[test]
    fn sub_genres_nest_recursively() {
        let genre: Genre = serde_json::from_str(
            r#"{"id":"rock","subGenres":[{"id":"punk","subGenres":[{"id":"hardcore"}]}]}"#,
        )
        .unwrap();
        assert_eq!(genre.sub_genres[0].id.as_deref(), Some("punk"));
        assert_eq!(
            genre.sub_genres[0].sub_genres[0].id.as_deref(),
            Some("hardcore")
        );
    }

    #[test]
    fn genre_models_survive_an_empty_object() {
        serde_json::from_str::<Genre>("{}").unwrap();
        serde_json::from_str::<GenreTitle>("{}").unwrap();
    }
}
