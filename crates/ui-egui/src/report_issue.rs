//! Help ▸ Report an Issue…: opens GitHub's new-issue form with the version and system details
//! filled in. The user reads every field there and submits it; PdfCraft sends nothing itself.
//!
//! The address uses the same form field ids as the other ArtCraft apps (`version`, `os`,
//! `system-info`, `document-info`), so one maintainer can read a report from any of them. What goes
//! in it is limited to facts about the app and the machine plus the open document's *type*. Never a
//! file name, a path, or anything from the document.

use pdfcraft_engine::links::NEW_ISSUE;
use std::fmt::Write as _;

/// The longest any one prefilled value, other than the system block, may be, in UTF-8 bytes.
const VALUE_CAP: usize = 80;
/// The longest the system block may be, in UTF-8 bytes.
const SYSTEM_INFO_CAP: usize = 400;

/// What the report says about the app, the machine and the open document.
pub(crate) struct Facts {
    pub version: String,
    pub os: String,
    pub system_info: String,
    pub document_info: String,
}

impl Facts {
    /// `graphics` is the adapter the window is drawn with (`None` when the host didn't say).
    /// `has_document` is whether a PDF is open; its name and contents never enter a report.
    pub fn new(graphics: Option<&str>, pixels_per_point: f32, has_document: bool) -> Self {
        let build = if cfg!(debug_assertions) { "debug" } else { "release" };
        let graphics = bounded(graphics.unwrap_or("unknown"), VALUE_CAP);
        let system_info = format!(
            "Build: {build}\nGraphics: {graphics}\nInterface language: {}\nDisplay scale: {pixels_per_point:.2}",
            crate::i18n::current().code()
        );
        Self {
            version: bounded(&format!("PdfCraft {}", pdfcraft_engine::build_info::version()), VALUE_CAP).to_owned(),
            os: bounded(&operating_system(), VALUE_CAP).to_owned(),
            system_info: bounded(&system_info, SYSTEM_INFO_CAP).to_owned(),
            document_info: format!("Document type: {}", if has_document { "pdf" } else { "none" }),
        }
    }

    /// The address that opens the prefilled form.
    pub fn url(&self) -> String {
        let fields = [
            ("version", self.version.as_str()),
            ("os", self.os.as_str()),
            ("system-info", self.system_info.as_str()),
            ("document-info", self.document_info.as_str()),
        ];
        let mut url = format!("{NEW_ISSUE}?template=bug_report.yml");
        for (id, value) in fields {
            url.push('&');
            url.push_str(id);
            url.push('=');
            encode(value, &mut url);
        }
        url
    }

    /// The same facts as plain text, for pasting into the form by hand.
    pub fn details(&self) -> String {
        format!("Version: {}\nOS: {}\n{}\n{}", self.version, self.os, self.system_info, self.document_info)
    }
}

/// `Windows x86_64`, `macOS aarch64`, or `Web` for the browser build. No OS version: that would
/// take a dependency, and the form asks for it.
fn operating_system() -> String {
    if cfg!(target_arch = "wasm32") {
        return "Web".to_owned();
    }
    let os = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        "freebsd" => "FreeBSD",
        "openbsd" => "OpenBSD",
        "netbsd" => "NetBSD",
        other => other,
    };
    format!("{os} {}", std::env::consts::ARCH)
}

/// `value` cut to at most `limit` bytes, at a character boundary.
fn bounded(value: &str, limit: usize) -> &str {
    if value.len() <= limit {
        return value;
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.get(..end).unwrap_or_default()
}

/// Appends `value` percent-encoded, keeping only the RFC 3986 unreserved characters.
fn encode(value: &str, out: &mut String) {
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}"); // writing to a String cannot fail
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(url: &str) -> Vec<(&str, &str)> {
        let (_, query) = url.split_once('?').expect("a query");
        query.split('&').map(|p| p.split_once('=').expect("name=value")).collect()
    }

    #[test]
    fn the_address_is_the_new_issue_form_with_the_four_fields() {
        let url = Facts::new(Some("Test GPU (Vulkan, DiscreteGpu)"), 1.5, true).url();
        assert!(url.starts_with("https://github.com/storytold/pdfcraft/issues/new?"), "{url}");
        let names: Vec<_> = params(&url).iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ["template", "version", "os", "system-info", "document-info"]);
        assert!(url.contains("template=bug_report.yml&"));
        assert!(url.ends_with("&document-info=Document%20type%3A%20pdf"), "{url}");
    }

    #[test]
    fn the_system_block_has_a_line_per_fact_in_a_fixed_order() {
        let facts = Facts::new(Some("Test GPU (Vulkan, DiscreteGpu)"), 1.5, false);
        let keys: Vec<_> = facts.system_info.lines().filter_map(|l| l.split_once(": ").map(|(k, _)| k)).collect();
        assert_eq!(keys, ["Build", "Graphics", "Interface language", "Display scale"]);
        assert!(facts.system_info.contains("Graphics: Test GPU (Vulkan, DiscreteGpu)"));
        assert!(facts.system_info.contains("Display scale: 1.50"));
        assert_eq!(facts.document_info, "Document type: none");
        assert_eq!(Facts::new(None, 1.0, false).system_info.lines().nth(1), Some("Graphics: unknown"));
    }

    #[test]
    fn hostile_characters_stay_inside_their_own_parameter() {
        let hostile = "a&b=c?d#e%f\nline\u{4e2d}\u{6587} \"q\" <x>";
        let mut facts = Facts::new(Some(hostile), 1.0, true);
        facts.version = hostile.to_owned();
        facts.os = hostile.to_owned();
        let url = facts.url();
        let query = url.split_once('?').expect("a query").1;
        assert_eq!(query.matches('&').count(), 4, "{url}");
        assert_eq!(query.matches('=').count(), 5, "{url}");
        assert!(!url.contains('#') && !url.contains(' ') && !url.contains('\n') && !url.contains('<') && !url.contains('"'), "{url}");
        // Only unreserved characters and percent escapes survive in a value.
        for (_, value) in params(&url).into_iter().skip(1) {
            assert!(value.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.~%".contains(&b)), "{value}");
        }
    }

    fn huge_version() -> String {
        "\u{4e2d}".repeat(10_000)
    }

    #[test]
    fn long_values_are_cut_on_a_character_boundary_and_the_address_stays_short() {
        let huge = "\u{4e2d}".repeat(10_000);
        let facts = Facts::new(Some(&huge), 1.0, true);
        assert!(facts.version.len() <= VALUE_CAP && facts.os.len() <= VALUE_CAP);
        assert!(facts.system_info.len() <= SYSTEM_INFO_CAP);
        // The cut graphics name is still whole characters.
        assert!(facts.system_info.contains('\u{4e2d}'));
        let mut facts = facts;
        facts.version = bounded(&huge_version(), VALUE_CAP).to_owned();
        facts.os = facts.version.clone();
        let url = facts.url();
        assert!(url.len() < 2048, "{} bytes", url.len());
    }

    #[test]
    fn bounded_never_splits_a_character() {
        assert_eq!(bounded("abc", 5), "abc");
        assert_eq!(bounded("abc", 2), "ab");
        assert_eq!(bounded("\u{e9}\u{e9}", 3), "\u{e9}");
        assert_eq!(bounded("\u{4e2d}", 2), "");
        assert_eq!(bounded("abc", 0), "");
    }

    #[test]
    fn the_details_text_repeats_what_the_form_gets() {
        let facts = Facts::new(Some("GPU"), 2.0, true);
        let text = facts.details();
        for part in [&facts.version, &facts.os, &facts.system_info, &facts.document_info] {
            assert!(text.contains(part.as_str()), "{text}");
        }
        assert!(text.starts_with("Version: PdfCraft "));
    }
}
