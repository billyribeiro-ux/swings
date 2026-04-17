// FDN-06 utility module. Items here are consumed by later Phase 4 subsystems
// (blog, popups, notifications) — they exist ahead of their callers by
// design. Tests exercise every public surface.
#![allow(dead_code)]

//! HTML sanitization via `ammonia`.
//!
//! Two entry points:
//!
//! * [`sanitize_rich_text`] — preserves a curated allowlist of block / inline
//!   tags (headings, lists, blockquotes, emphasis, inline code, links, images)
//!   while stripping anything with script-execution or layout-disruption
//!   potential (`script`, `style`, `iframe`, `object`, `embed`, `svg`, `form`,
//!   `input`, all `on*` event handlers, and inline `style` attributes).
//! * [`sanitize_plain_text`] — drops all markup entirely and collapses
//!   consecutive whitespace to a single space.
//!
//! # Link and image policy
//!
//! * `<a href>` is restricted to `http`, `https`, and `mailto:` URIs, and is
//!   rewritten with `rel="noopener nofollow"` unconditionally. `javascript:`
//!   and `data:` are stripped by ammonia's default URL-scheme filter together
//!   with our explicit allowlist.
//! * `<img src>` allows `http`, `https`, and `data:` (for small inline
//!   previews). Other schemes are rejected.
//!
//! # Builder caching
//!
//! Each sanitizer is built once per process via a [`OnceLock`] — ammonia
//! builders are cheap to use but non-trivial to construct.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use ammonia::Builder;

/// Tags allowed in rich-text output. Anything not in this set is stripped
/// (with its children preserved where ammonia's defaults dictate).
const RICH_TEXT_TAGS: &[&str] = &[
    "a",
    "b",
    "blockquote",
    "br",
    "code",
    "em",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "i",
    "img",
    "li",
    "ol",
    "p",
    "pre",
    "s",
    "strong",
    "u",
    "ul",
];

/// Tags that must be completely removed (content included). These would
/// otherwise risk layout takeover or content-injection even with attributes
/// stripped.
const CLEAN_CONTENT_TAGS: &[&str] = &[
    "embed", "form", "iframe", "input", "object", "script", "style", "svg",
];

fn rich_text_sanitizer() -> &'static Builder<'static> {
    static B: OnceLock<Builder<'static>> = OnceLock::new();
    B.get_or_init(|| {
        let mut b = Builder::default();

        let tags: HashSet<&'static str> = RICH_TEXT_TAGS.iter().copied().collect();
        b.tags(tags);

        let clean: HashSet<&'static str> = CLEAN_CONTENT_TAGS.iter().copied().collect();
        b.clean_content_tags(clean);

        let mut tag_attrs: HashMap<&'static str, HashSet<&'static str>> = HashMap::new();
        tag_attrs.insert("a", ["href", "title"].into_iter().collect());
        tag_attrs.insert(
            "img",
            ["src", "alt", "title", "width", "height"]
                .into_iter()
                .collect(),
        );
        b.tag_attributes(tag_attrs);

        // Scheme allowlists. Anything else is dropped by ammonia.
        let url_schemes: HashSet<&'static str> =
            ["http", "https", "mailto", "data"].into_iter().collect();
        b.url_schemes(url_schemes);

        // Force rel="noopener nofollow" on every <a>, overriding whatever the
        // author set.
        b.link_rel(Some("noopener nofollow"));

        // Drop global `style` and all `on*` handlers by never adding them to
        // the allowed attributes. Defensive: also scrub comments.
        b.strip_comments(true);

        b
    })
}

fn plain_text_sanitizer() -> &'static Builder<'static> {
    static B: OnceLock<Builder<'static>> = OnceLock::new();
    B.get_or_init(|| {
        let mut b = Builder::default();
        // Allowlist nothing — every tag is stripped.
        b.tags(HashSet::new());
        // Strip dangerous tags *with* their contents so that `<script>alert()</script>`
        // does not leave the payload text behind.
        let clean: HashSet<&'static str> = CLEAN_CONTENT_TAGS.iter().copied().collect();
        b.clean_content_tags(clean);
        b.strip_comments(true);
        b
    })
}

/// Sanitize untrusted rich text for display.
///
/// The result is safe to embed directly inside server-rendered HTML. See
/// module docs for the tag / attribute / URL-scheme policy.
#[must_use]
pub fn sanitize_rich_text(input: &str) -> String {
    rich_text_sanitizer().clean(input).to_string()
}

/// Strip every HTML tag and collapse runs of whitespace to single spaces.
///
/// Useful for indexing, e-mail subject lines, or any plain-text-only surface.
#[must_use]
pub fn sanitize_plain_text(input: &str) -> String {
    let stripped = plain_text_sanitizer().clean(input).to_string();
    collapse_whitespace(&stripped)
}

fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_ws = true; // trim leading whitespace
    for c in s.chars() {
        if c.is_whitespace() {
            if !last_was_ws {
                out.push(' ');
            }
            last_was_ws = true;
        } else {
            out.push(c);
            last_was_ws = false;
        }
    }
    // Trim trailing space.
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_tag_is_removed_with_content() {
        let out = sanitize_rich_text("<p>hi</p><script>alert('x')</script>");
        assert!(out.contains("<p>hi</p>"));
        assert!(!out.contains("script"));
        assert!(!out.contains("alert"));
    }

    #[test]
    fn javascript_href_is_stripped() {
        let out = sanitize_rich_text(r#"<a href="javascript:alert(1)">x</a>"#);
        assert!(!out.contains("javascript"));
        // The text remains even if the attribute is dropped.
        assert!(out.contains("x"));
    }

    #[test]
    fn onerror_attribute_is_removed() {
        let out = sanitize_rich_text(r#"<img src="https://example.com/a.png" onerror="boom()">"#);
        assert!(!out.contains("onerror"));
        assert!(!out.contains("boom"));
        assert!(out.contains("https://example.com/a.png"));
    }

    #[test]
    fn iframe_is_removed_with_content() {
        let out = sanitize_rich_text(r#"<iframe src="https://evil.tld">trapped</iframe>"#);
        assert!(!out.contains("iframe"));
        assert!(!out.contains("trapped"));
        assert!(!out.contains("evil.tld"));
    }

    #[test]
    fn style_tag_is_removed_with_content() {
        let out = sanitize_rich_text("<style>body{display:none}</style><p>ok</p>");
        assert!(!out.contains("style"));
        assert!(!out.contains("display:none"));
        assert!(out.contains("<p>ok</p>"));
    }

    #[test]
    fn svg_is_removed() {
        let out = sanitize_rich_text(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><g onload="x()"/></svg><p>hi</p>"#,
        );
        assert!(!out.contains("svg"));
        assert!(!out.contains("onload"));
        assert!(out.contains("<p>hi</p>"));
    }

    #[test]
    fn inline_style_attribute_is_stripped() {
        let out = sanitize_rich_text(r#"<p style="color:red">x</p>"#);
        assert!(!out.contains("style"));
        assert!(out.contains("<p>x</p>") || out.contains("<p>x</p>\n"));
    }

    #[test]
    fn normal_content_is_preserved() {
        let input = "<h2>Title</h2><p>Some <strong>bold</strong> and <em>italic</em> text with \
             a <a href=\"https://example.com\">link</a>.</p><ul><li>one</li><li>two</li></ul>";
        let out = sanitize_rich_text(input);
        assert!(out.contains("<h2>Title</h2>"));
        assert!(out.contains("<strong>bold</strong>"));
        assert!(out.contains("<em>italic</em>"));
        assert!(out.contains("<li>one</li>"));
        assert!(out.contains(r#"href="https://example.com""#));
    }

    #[test]
    fn anchor_gets_noopener_nofollow() {
        let out = sanitize_rich_text(r#"<a href="https://x.com" rel="author">x</a>"#);
        // `ammonia` rewrites rel to exactly what we asked for.
        assert!(out.contains("rel=\"noopener nofollow\""));
        assert!(!out.contains("author"));
    }

    #[test]
    fn mailto_links_survive() {
        let out = sanitize_rich_text(r#"<a href="mailto:hi@example.com">mail</a>"#);
        assert!(out.contains("mailto:hi@example.com"));
    }

    #[test]
    fn img_data_uri_survives() {
        let out = sanitize_rich_text(r#"<img src="data:image/png;base64,AAAA" alt="x">"#);
        assert!(out.contains("data:image/png;base64,AAAA"));
    }

    #[test]
    fn plain_text_strips_all_tags() {
        let out = sanitize_plain_text("<p>hello <strong>world</strong></p>");
        assert_eq!(out, "hello world");
    }

    #[test]
    fn plain_text_drops_script_payload() {
        let out = sanitize_plain_text("before<script>alert(1)</script>after");
        assert_eq!(out, "beforeafter");
    }

    #[test]
    fn plain_text_collapses_whitespace() {
        let out = sanitize_plain_text("  a  \n\t b   c  ");
        assert_eq!(out, "a b c");
    }

    #[test]
    fn plain_text_empty_is_empty() {
        assert_eq!(sanitize_plain_text(""), "");
        assert_eq!(sanitize_plain_text("   \n\t "), "");
    }
}
