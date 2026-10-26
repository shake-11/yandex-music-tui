//! request parameter building: query strings, form bodies and path segments.
//!
//! the api takes the same key/value shape in a query and in a form body, and
//! repeats a key per element for list values (`track-ids=1&track-ids=2`).
//! [`Params`] is the one builder both encodings go through. the third place a
//! caller's value reaches the wire — the url path — is covered by an internal
//! `segment` helper, which every endpoint runs its path parameters through.

use std::fmt::Display;
use std::fmt::Write as _;

/// an ordered set of request parameters.
///
/// `None` values are dropped rather than sent empty, matching what the api
/// expects for optional parameters.
#[derive(Debug, Default, Clone)]
pub struct Params {
    pairs: Vec<(String, String)>,
}

impl Params {
    /// start an empty parameter set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// append a parameter.
    #[must_use]
    pub fn add(mut self, key: &str, value: impl Display) -> Self {
        self.pairs.push((key.to_owned(), value.to_string()));
        self
    }

    /// append a parameter only when the value is present.
    #[must_use]
    pub fn maybe(self, key: &str, value: Option<impl Display>) -> Self {
        match value {
            Some(v) => self.add(key, v),
            None => self,
        }
    }

    /// append one entry per element, repeating the key.
    #[must_use]
    pub fn add_all(mut self, key: &str, values: impl IntoIterator<Item = impl Display>) -> Self {
        for value in values {
            self.pairs.push((key.to_owned(), value.to_string()));
        }
        self
    }

    /// whether no parameters were added.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// the parameters as borrowed key/value pairs, in insertion order.
    pub(crate) fn pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.pairs.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

impl FromIterator<(String, String)> for Params {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        Self {
            pairs: iter.into_iter().collect(),
        }
    }
}

/// percent-encode a value for use as a single url path segment.
///
/// plenty of path parameters are opaque strings the caller supplies — a playlist
/// uuid, a station `type:tag` pair, a queue id, a chart name. a `/`, `?` or `#`
/// in one of those would silently retarget the request at a different endpoint,
/// so anything that is not a legal path character is escaped here.
///
/// "legal" is rfc 3986's `pchar`, which is wider than the unreserved set: `:`,
/// `@` and the sub-delimiters stay literal because the api's own identifiers use
/// them (`genre:pop`, `1234:56`) and escaping those would change what the server
/// routes on.
pub(crate) fn segment(value: impl Display) -> String {
    let value = value.to_string();
    // the common case is a numeric id, which needs nothing done to it
    if value.bytes().all(is_pchar) {
        return value;
    }

    let mut out = String::with_capacity(value.len() + 8);
    for byte in value.bytes() {
        if is_pchar(byte) {
            out.push(byte as char);
        } else {
            // writing to a string cannot fail
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// whether a byte may appear literally in a path segment (rfc 3986 `pchar`).
const fn is_pchar(byte: u8) -> bool {
    matches!(byte,
        // unreserved
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
        // sub-delims
        | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
        // and these two, which `pchar` adds on top
        | b':' | b'@'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(params: &Params) -> Vec<(String, String)> {
        params
            .pairs()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }

    #[test]
    fn insertion_order_is_preserved() {
        let params = Params::new().add("b", 2).add("a", 1).add("c", 3);
        assert_eq!(
            collect(&params),
            vec![
                ("b".into(), "2".into()),
                ("a".into(), "1".into()),
                ("c".into(), "3".into()),
            ]
        );
    }

    #[test]
    fn maybe_drops_absent_values() {
        let params = Params::new()
            .maybe("kept", Some("yes"))
            .maybe("dropped", Option::<&str>::None);
        assert_eq!(collect(&params), vec![("kept".into(), "yes".into())]);
    }

    #[test]
    fn add_all_repeats_the_key_per_element() {
        let params = Params::new().add_all("track-ids", [1, 2, 3]);
        assert_eq!(
            collect(&params),
            vec![
                ("track-ids".into(), "1".into()),
                ("track-ids".into(), "2".into()),
                ("track-ids".into(), "3".into()),
            ]
        );
    }

    #[test]
    fn add_all_over_an_empty_iterator_adds_nothing() {
        assert!(Params::new().add_all("ids", Vec::<i64>::new()).is_empty());
    }

    #[test]
    fn a_clean_segment_passes_through_untouched() {
        assert_eq!(segment(42), "42");
        assert_eq!(segment("daily"), "daily");
        // the api's own compound identifiers must survive verbatim, or the server
        // routes on something other than what was asked for
        assert_eq!(segment("genre:pop"), "genre:pop");
        assert_eq!(segment("1234:56"), "1234:56");
        assert_eq!(segment("new-releases"), "new-releases");
        assert_eq!(
            segment("d4a1f7e2-0000-4c1a-9f3b-000000000000"),
            "d4a1f7e2-0000-4c1a-9f3b-000000000000"
        );
    }

    #[test]
    fn a_separator_in_a_segment_cannot_retarget_the_request() {
        // the whole point: this must not become a request to /account/status
        assert_eq!(
            segment("x/../../account/status"),
            "x%2F..%2F..%2Faccount%2Fstatus"
        );
        assert_eq!(segment("a?b=c"), "a%3Fb=c");
        assert_eq!(segment("a#frag"), "a%23frag");
        // a pre-escaped value is escaped again rather than decoded twice
        assert_eq!(segment("a%2Fb"), "a%252Fb");
    }

    #[test]
    fn whitespace_and_non_ascii_are_escaped() {
        assert_eq!(segment("a b"), "a%20b");
        assert_eq!(segment("тест"), "%D1%82%D0%B5%D1%81%D1%82");
        assert_eq!(segment("a\nb"), "a%0Ab");
        assert_eq!(segment(""), "");
    }

    #[test]
    fn booleans_render_as_the_api_expects() {
        let params = Params::new()
            .add("nocorrect", false)
            .add("with-positions", true);
        assert_eq!(
            collect(&params),
            vec![
                ("nocorrect".into(), "false".into()),
                ("with-positions".into(), "true".into()),
            ]
        );
    }
}
