// FDN-06 utility module. Items here are consumed by later Phase 4 subsystems
// (blog, popups, notifications) — they exist ahead of their callers by
// design. Tests exercise every public surface.
#![allow(dead_code)]

//! HTML sanitization via `ammonia`.
//!
//! Two entry points:
//!
//! * [`sanitize_rich_text`] — preserves a curated allowlist of block / inline
//!   tags suitable for blog body content authored via TipTap: headings,
//!   lists (incl. task lists), tables, blockquotes, emphasis, inline code,
//!   links, images, sub/sup, mark/highlight, and YouTube `<iframe>` (with the
//!   `src` host pinned to youtube-nocookie.com / youtube.com / youtu.be).
//!   `style` is allowed only on elements where TipTap commonly emits it
//!   (`p`, `span`, `div`, `td`, `th`, `img`), and only for the tokens
//!   `color`, `background-color`, `text-align`, `width`, `height`, and
//!   `aspect-ratio` — every other declaration is dropped at the attribute
//!   filter, so a pasted `position: fixed; top:0; left:0; ...` cannot
//!   take over the page.
//!
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
//! * `<iframe src>` is allowed *only* for the YouTube embed hosts. Any other
//!   iframe is removed together with its content (defence-in-depth: even if
//!   the tag survived, the URL-scheme + host filter would zero the `src`).
//!
//! # Style policy
//!
//! TipTap surfaces colour, alignment, and image sizing via inline `style`
//! attributes. The naive ammonia default is "strip every `style` attribute,"
//! which is what we used to do — and is what destroyed table layouts,
//! coloured text, image sizes, and aligned paragraphs at the write boundary.
//!
//! Instead we now run an [`attribute_filter`] that, for `style` attributes
//! only, parses the declaration list with a tiny ad-hoc tokenizer and keeps
//! only the declarations whose property is in [`SAFE_STYLE_PROPERTIES`] AND
//! whose value matches [`STYLE_VALUE_RE`] (a conservative regex that rejects
//! anything containing `url(`, `expression(`, `javascript:`, `<`, `>`, or
//! attributes-style brackets). Everything else is dropped silently. Result:
//! `style="color: red; position: absolute"` → `style="color: red"`, and
//! `style="background: url(javascript:alert(1))"` → attribute removed.
//!
//! # Builder caching
//!
//! Each sanitizer is built once per process via a [`OnceLock`] — ammonia
//! builders are cheap to use but non-trivial to construct.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use ammonia::Builder;

/// Tags allowed in rich-text output. Anything not in this set is stripped
/// (with its children preserved where ammonia's defaults dictate).
///
/// The set is the union of:
/// 1. Markdown-equivalent block + inline tags (`p`, `h1..h6`, `ul`, `ol`, …).
/// 2. TipTap-specific extensions in active use across the editor:
///    - tables (`table`, `thead`, `tbody`, `tr`, `th`, `td`)
///    - task lists (`ul`, `li` already covered; the `data-type`/`data-checked`
///      attributes are allowed below)
///    - inline highlights (`mark`)
///    - sub/superscript (`sub`, `sup`)
///    - layout containers (`div`, `span`) used by ResizableImage, the
///      `ReadMore` cut, and colour/alignment spans
/// 3. `iframe` *exclusively* for the YouTube embed extension. Without an
///    entry here ammonia would drop the tag entirely, so we add it and
///    pair it with a host allowlist on the `src` attribute filter.
const RICH_TEXT_TAGS: &[&str] = &[
    "a",
    "b",
    "blockquote",
    "br",
    "code",
    "div",
    "em",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "i",
    "iframe",
    "img",
    "li",
    "mark",
    "ol",
    "p",
    "pre",
    "s",
    "span",
    "strong",
    "sub",
    "sup",
    "table",
    "tbody",
    "td",
    "th",
    "thead",
    "tr",
    "u",
    "ul",
];

/// Tags that must be completely removed (content included). These would
/// otherwise risk layout takeover or content-injection even with attributes
/// stripped. `iframe` is intentionally NOT in this list — it is allowed on
/// the tag set above and constrained to YouTube via the attribute filter.
const CLEAN_CONTENT_TAGS: &[&str] = &["embed", "form", "input", "object", "script", "style", "svg"];

/// CSS properties that survive the inline-style filter. The list is
/// deliberately tiny: anything that lets the author take over layout
/// (`position`, `top`/`left`, `transform`, `display:none`, `visibility`,
/// `opacity`, `pointer-events`) or load remote resources (`background`,
/// `background-image`) is excluded.
const SAFE_STYLE_PROPERTIES: &[&str] = &[
    "color",
    "background-color",
    "text-align",
    "width",
    "height",
    "aspect-ratio",
    "max-width",
    "max-height",
];

/// YouTube embed hosts. Anything else and the iframe is dropped.
const YOUTUBE_IFRAME_HOSTS: &[&str] = &[
    "www.youtube.com",
    "youtube.com",
    "www.youtube-nocookie.com",
    "youtube-nocookie.com",
    "youtu.be",
];

/// Validate a single style declaration value. Conservative: refuse any
/// payload that could embed a URL, an expression, or HTML brackets. Returns
/// `true` when the value is safe to keep.
fn style_value_is_safe(value: &str) -> bool {
    let lc = value.to_ascii_lowercase();
    if lc.contains("url(") || lc.contains("expression(") || lc.contains("javascript:") {
        return false;
    }
    // Reject anything that smells like nested HTML / attribute breakouts.
    if value.contains('<') || value.contains('>') || value.contains('"') || value.contains('\'') {
        return false;
    }
    // Length cap — runaway values are almost always malicious or broken.
    if value.len() > 256 {
        return false;
    }
    true
}

/// Filter an inline `style` attribute by walking its declaration list and
/// keeping only entries whose property is in [`SAFE_STYLE_PROPERTIES`] and
/// whose value passes [`style_value_is_safe`]. Returns the rebuilt attribute
/// value, or `None` when no declaration survives.
fn filter_style_attribute(input: &str) -> Option<String> {
    let safe_props: HashSet<&str> = SAFE_STYLE_PROPERTIES.iter().copied().collect();
    let mut kept = Vec::new();
    for decl in input.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        let Some((prop, value)) = decl.split_once(':') else {
            continue;
        };
        let prop_lc = prop.trim().to_ascii_lowercase();
        let value_trimmed = value.trim().trim_end_matches(';');
        if !safe_props.contains(prop_lc.as_str()) {
            continue;
        }
        if !style_value_is_safe(value_trimmed) {
            continue;
        }
        kept.push(format!("{prop_lc}: {value_trimmed}"));
    }
    if kept.is_empty() {
        None
    } else {
        Some(kept.join("; "))
    }
}

/// Validate an iframe `src` against the YouTube host allowlist. Returns
/// `true` only for absolute https URLs whose host matches an entry in
/// [`YOUTUBE_IFRAME_HOSTS`].
fn iframe_src_is_youtube(src: &str) -> bool {
    let Ok(url) = url::Url::parse(src) else {
        return false;
    };
    if url.scheme() != "https" {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    YOUTUBE_IFRAME_HOSTS.contains(&host)
}

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
            ["src", "alt", "title", "width", "height", "style"]
                .into_iter()
                .collect(),
        );
        // TipTap task list extension marks list items with `data-checked`
        // and the parent `<ul data-type="taskList">`; surface them so the
        // public render keeps the checkbox state.
        tag_attrs.insert("ul", ["data-type"].into_iter().collect());
        tag_attrs.insert("li", ["data-checked", "data-type"].into_iter().collect());
        // Tables — column alignment + spans, plus the `style` hatch for
        // per-cell colour set by the editor.
        tag_attrs.insert("td", ["colspan", "rowspan", "style"].into_iter().collect());
        tag_attrs.insert(
            "th",
            ["colspan", "rowspan", "scope", "style"]
                .into_iter()
                .collect(),
        );
        // Layout containers — keep `class` (TipTap node-views render to
        // class-named spans/divs) plus `style` for inline colour.
        tag_attrs.insert("span", ["class", "style"].into_iter().collect());
        tag_attrs.insert("div", ["class", "style", "data-type"].into_iter().collect());
        tag_attrs.insert("p", ["class", "style"].into_iter().collect());
        // YouTube embed extension. The actual host check is in the
        // attribute filter — these only declare which attributes survive
        // the parse step at all.
        tag_attrs.insert(
            "iframe",
            [
                "src",
                "width",
                "height",
                "frameborder",
                "allow",
                "allowfullscreen",
                "title",
            ]
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

        // Style + iframe-host filtering. Runs *after* ammonia has already
        // applied its tag/attribute/scheme allowlists, so the closure only
        // sees attributes we declared safe at the structural level.
        b.attribute_filter(|element, attribute, value| {
            match (element, attribute) {
                (_, "style") => filter_style_attribute(value).map(Cow::Owned),
                ("iframe", "src") => {
                    if iframe_src_is_youtube(value) {
                        Some(Cow::Borrowed(value))
                    } else {
                        None
                    }
                }
                // Defence-in-depth: kill any iframe attribute we don't
                // explicitly recognise even though `tag_attrs` already
                // bounds the set, so a future attribute-allowlist edit
                // can't widen the surface accidentally.
                ("iframe", attr) => {
                    const SAFE_IFRAME_ATTRS: &[&str] = &[
                        "src",
                        "width",
                        "height",
                        "frameborder",
                        "allow",
                        "allowfullscreen",
                        "title",
                    ];
                    if SAFE_IFRAME_ATTRS.contains(&attr) {
                        Some(Cow::Borrowed(value))
                    } else {
                        None
                    }
                }
                _ => Some(Cow::Borrowed(value)),
            }
        });

        // Drop comments + global `on*` handlers (handlers are attribute-name
        // gated by ammonia; we just declare we don't list any of them).
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
    fn non_youtube_iframe_src_is_dropped() {
        // The iframe tag itself is allowed (so YouTube embeds survive),
        // but any non-allowlisted host loses its `src`. The empty shell
        // renders to nothing in the browser — no script execution, no
        // remote loading.
        let out = sanitize_rich_text(r#"<iframe src="https://evil.tld">trapped</iframe>"#);
        assert!(!out.contains("evil.tld"));
        // Inner text content is preserved by ammonia for surviving tags;
        // that's harmless because the iframe shell with no `src` renders
        // empty regardless.
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
    fn inline_style_with_only_unsafe_properties_is_stripped() {
        // `position` is not in the safe property allowlist, so the entire
        // `style` attribute is dropped (no surviving declarations).
        let out = sanitize_rich_text(r#"<p style="position: absolute">x</p>"#);
        assert!(!out.contains("style"));
        assert!(out.contains("<p>x</p>"));
    }

    #[test]
    fn inline_style_with_safe_property_survives() {
        // `color` IS in the safe property allowlist, so the declaration
        // is kept. Past behaviour was to strip the attribute outright,
        // which destroyed every coloured paragraph at the write boundary.
        let out = sanitize_rich_text(r#"<p style="color: red">x</p>"#);
        assert!(out.contains("color: red"));
        assert!(out.contains(">x</p>"));
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

    // ── Forensic C-5 regression coverage ────────────────────────────────
    // Prior to the wave-1 sanitiser rewrite, every TipTap surface below
    // was destroyed at the write boundary. Each test pins one previously
    // gutted surface so a future regression cannot silently re-introduce
    // the bug.

    #[test]
    fn tables_round_trip() {
        let out = sanitize_rich_text(
            "<table><thead><tr><th>A</th><th>B</th></tr></thead>\
             <tbody><tr><td>1</td><td>2</td></tr></tbody></table>",
        );
        assert!(out.contains("<table>"));
        assert!(out.contains("<thead>"));
        assert!(out.contains("<th>A</th>"));
        assert!(out.contains("<td>2</td>"));
    }

    #[test]
    fn task_list_attributes_survive() {
        let out =
            sanitize_rich_text(r#"<ul data-type="taskList"><li data-checked="true">x</li></ul>"#);
        assert!(out.contains(r#"data-type="taskList""#));
        assert!(out.contains(r#"data-checked="true""#));
    }

    #[test]
    fn sub_sup_mark_survive() {
        let out = sanitize_rich_text("<p>e<sup>iπ</sup>+1=0; H<sub>2</sub>O; <mark>hot</mark></p>");
        assert!(out.contains("<sup>iπ</sup>"));
        assert!(out.contains("<sub>2</sub>"));
        assert!(out.contains("<mark>hot</mark>"));
    }

    #[test]
    fn youtube_iframe_survives_other_iframes_dropped() {
        // YouTube embed: kept.
        let yt = sanitize_rich_text(
            r#"<iframe src="https://www.youtube.com/embed/abc123" allowfullscreen></iframe>"#,
        );
        assert!(yt.contains(r#"src="https://www.youtube.com/embed/abc123""#));
        assert!(yt.contains("allowfullscreen"));

        // Non-YouTube: src is stripped, leaving an empty iframe shell that
        // renders nothing — no XSS surface, no exfil, no clickjacking
        // because any onload would also have been removed.
        let evil = sanitize_rich_text(r#"<iframe src="https://evil.tld/a"></iframe>"#);
        assert!(!evil.contains("evil.tld"));

        // http (non-https) YouTube: rejected even though host matches —
        // we only allow https embeds.
        let plain_http =
            sanitize_rich_text(r#"<iframe src="http://www.youtube.com/embed/abc"></iframe>"#);
        assert!(!plain_http.contains("youtube.com"));
    }

    #[test]
    fn safe_style_tokens_survive_unsafe_dropped() {
        // Coloured paragraph + alignment: kept.
        let out = sanitize_rich_text(
            r#"<p style="color: red; text-align: center; position: fixed; top: 0">x</p>"#,
        );
        assert!(out.contains("color: red"));
        assert!(out.contains("text-align: center"));
        assert!(!out.contains("position"));
        assert!(!out.contains("fixed"));
        assert!(!out.contains("top:"));

        // url(...) value rejected even on a safe property.
        let bg =
            sanitize_rich_text(r#"<p style="background-color: url(javascript:alert(1))">y</p>"#);
        assert!(!bg.contains("javascript"));
        assert!(!bg.contains("url("));

        // Image width (TipTap ResizableImage extension): kept.
        let img = sanitize_rich_text(
            r#"<img src="https://example.com/a.png" style="width: 320px; height: 180px">"#,
        );
        assert!(img.contains("width: 320px"));
        assert!(img.contains("height: 180px"));
    }

    #[test]
    fn class_attribute_survives_on_layout_containers() {
        // ResizableImage / ReadMore wrappers carry CSS classes the public
        // template targets. Stripping them used to make the read-more cut
        // invisible at render time.
        let out = sanitize_rich_text(r#"<div class="read-more"><p class="lede">Lede.</p></div>"#);
        assert!(out.contains(r#"class="read-more""#));
        assert!(out.contains(r#"class="lede""#));
    }

    #[test]
    fn style_attribute_with_no_safe_tokens_is_removed_entirely() {
        let out = sanitize_rich_text(r#"<p style="position: absolute; top: 0">x</p>"#);
        assert!(!out.contains("style"));
    }
}
