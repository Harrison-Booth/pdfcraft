//! Community links: the Discord button is one click away everywhere; Help menu, About dialog and
//! home screen open the ArtCraft and PdfCraft pages.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_engine::links;
use pdfcraft_ui_egui::{Dialog, PdfCraftApp};

fn harness(setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        setup(&mut app);
        app
    });
    h.run_steps(4);
    h
}

#[test]
fn discord_button_in_the_top_bar_opens_discord() {
    let mut h = harness(|_| {});
    h.get_by_label("Discord").click();
    h.run_steps(2);
    assert_eq!(h.state().last_opened_url.as_deref(), Some(links::DISCORD));
    assert_eq!(links::DISCORD, "https://discord.gg/artcraft");
}

#[test]
fn home_screen_links() {
    for (label, url) in [
        ("Join our Discord", links::DISCORD),
        ("PdfCraft web page", "https://getartcraft.com/apps/pdfcraft"),
        ("PdfCraft on GitHub", "https://github.com/storytold/pdfcraft"),
        ("ArtCraft website", "https://getartcraft.com"),
    ] {
        let mut h = harness(|_| {});
        h.get_by_label("Join the ArtCraft community");
        h.get_by_label(label).click();
        h.run_steps(2);
        assert_eq!(h.state().last_opened_url.as_deref(), Some(url), "{label}");
    }
}

#[test]
fn home_community_card_closes_for_this_run() {
    let mut h = harness(|_| {});
    h.get_by_label("Join the ArtCraft community");
    h.get_by_label("Close").click();
    h.run_steps(2);
    assert!(h.state().community_card_closed);
    assert!(h.query_by_label("Join the ArtCraft community").is_none());
    assert!(h.query_by_label("Join our Discord").is_none());
    // Not remembered: a fresh app shows the card again.
    assert!(!PdfCraftApp::new().community_card_closed);
}

#[test]
fn about_dialog_shows_the_brand_and_links() {
    let pdf = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";
    // With a document open, so the home screen's own links are not on screen.
    let mut h = harness(move |app| {
        app.open_bytes("one.pdf", None, pdf.to_vec()).unwrap();
        app.dialog = Some(Dialog::About);
    });
    assert!(h.query_all_by_label("ArtCraft").count() >= 2, "the mark and the wordmark (alt text)");
    h.get_by_label("Join our Discord").click();
    h.run_steps(2);
    assert_eq!(h.state().last_opened_url.as_deref(), Some(links::DISCORD));
}

#[test]
fn help_commands_open_each_link() {
    for l in links::LINKS {
        let mut h = harness(|_| {});
        assert!(h.state_mut().execute(l.command), "{}", l.command);
        assert_eq!(h.state().last_opened_url.as_deref(), Some(l.url));
        assert_eq!(pdfcraft_engine::commands::command(l.command).unwrap().menu, Some("Help"));
    }
}

/// Runs frames until the node labelled `label` exists and has stopped moving, then returns it.
///
/// The About dialog is a modal centred on its own size, and switching tabs changes that size, so
/// for a few frames afterwards the modal grows and recentres and every control in it moves. A
/// fixed number of frames is a guess at how long that takes: it was too few after the 0.5.0
/// contributors refresh (78 names instead of 5) on some machines, the click on Table landed where
/// the button had been, and the table never opened (#671).
fn settled<'a>(h: &'a mut Harness<'static, PdfCraftApp>, label: &'a str) -> egui_kittest::Node<'a> {
    let mut last = None;
    for _ in 0..60 {
        h.run_steps(1);
        let now = h.query_by_label(label).map(|n| n.rect());
        if now.is_some() && now == last {
            return h.get_by_label(label);
        }
        last = now;
    }
    panic!("{label:?} never appeared or never stopped moving");
}

#[test]
fn about_dialog_has_contributors_and_models_tabs() {
    let mut h = harness(|app| app.dialog = Some(Dialog::About));
    settled(&mut h, "Contributors").click();
    // The owner is always in the compiled-in credits (contributors/contributors.json), shown by username.
    settled(&mut h, "@echelon");
    settled(&mut h, "Table").click();
    settled(&mut h, "PRs");
    settled(&mut h, "Display name").click();
    settled(&mut h, "Brandon Thomas");
    settled(&mut h, "Models").click();
    h.run_steps(4);
    assert!(h.query_all_by_label("Anthropic").count() >= 1);
}

/// The parameter names of a Report an Issue… address, in order.
fn report_param_names(url: &str) -> Vec<&str> {
    url.split_once('?').expect("a query").1.split('&').map(|p| p.split_once('=').expect("name=value").0).collect()
}

/// The text copied by the last frames, if any (a click copies on the frame after it).
fn copied(h: &mut Harness<'static, PdfCraftApp>) -> Option<String> {
    for _ in 0..4 {
        h.run_steps(1);
        let found = h.output().platform_output.commands.iter().find_map(|c| match c {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

#[test]
fn report_an_issue_opens_the_prefilled_form_through_the_host_opener() {
    let opened = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let seen = opened.clone();
    let mut h = harness(move |app| {
        app.url_opener = Some(Box::new(move |url| {
            seen.borrow_mut().push(url.to_owned());
            Ok(())
        }))
    });
    assert!(h.state_mut().execute("help.report_issue"));
    h.run_steps(2);
    let opened = opened.borrow();
    assert_eq!(opened.len(), 1, "one address is opened, once");
    let url = &opened[0];
    assert!(url.starts_with(&format!("{}?template=bug_report.yml&", links::NEW_ISSUE)), "{url}");
    assert_eq!(report_param_names(url), ["template", "version", "os", "system-info", "document-info"]);
    assert_eq!(h.state().last_opened_url.as_deref(), Some(url.as_str()));
    assert!(h.state().failed_link.is_none(), "nothing extra shows when the browser opens");
    assert!(h.query_by_label("Couldn't open your web browser").is_none());
    // No document is open.
    assert!(url.ends_with("document-info=Document%20type%3A%20none"), "{url}");
}

#[test]
fn report_an_issue_without_a_host_opener_asks_egui_to_open_it() {
    let mut h = harness(|_| {});
    assert!(h.state_mut().execute("help.report_issue"));
    let url = h.state().last_opened_url.clone().expect("opened");
    assert!(url.starts_with(links::NEW_ISSUE), "{url}");
    assert!(h.state().failed_link.is_none());
}

#[test]
fn report_an_issue_never_includes_the_document_name_or_path() {
    let pdf = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";
    let mut h = harness(move |app| {
        app.open_bytes("Secret Budget.pdf", Some("/home/ann/Secret Budget.pdf".to_owned()), pdf.to_vec()).unwrap();
    });
    h.state_mut().execute("help.report_issue");
    let url = h.state().last_opened_url.clone().expect("opened");
    assert!(url.ends_with("document-info=Document%20type%3A%20pdf"), "{url}");
    let decoded = url.replace("%20", " ");
    for private in ["Secret", "Budget", "ann", "/home"] {
        assert!(!decoded.contains(private), "{private} is in {url}");
    }
}

#[test]
fn a_browser_that_does_not_open_shows_the_address_and_details_to_copy() {
    // With a document open, so the home screen's own Close button is not on screen.
    let pdf = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";
    let mut h = harness(move |app| {
        app.open_bytes("one.pdf", None, pdf.to_vec()).unwrap();
        app.url_opener = Some(Box::new(|_| Err("no browser is installed".to_owned())));
    });
    h.state_mut().execute("help.report_issue");
    h.run_steps(3);
    h.get_by_label("Couldn't open your web browser");
    h.get_by_label("no browser is installed");
    // Read-only fields, named by their labels for assistive technology.
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Address");
    h.get_by_role_and_label(egui::accesskit::Role::MultilineTextInput, "Details");
    let failed = h.state().failed_link.clone().expect("the dialog's data");
    assert_eq!(failed.url, links::NEW_ISSUE);
    assert!(failed.details.starts_with("Version: PdfCraft ") && failed.details.contains("Document type: pdf"), "{}", failed.details);
    h.get_by_label("Copy Details").click();
    let text = copied(&mut h).expect("Copy Details copies");
    assert_eq!(text, format!("{}\n\n{}", failed.details, links::NEW_ISSUE));
    assert!(h.state().failed_link.is_some(), "copying keeps the dialog open");
    h.get_by_label("Close").click();
    h.run_steps(3);
    assert!(h.state().failed_link.is_none());
    assert!(h.query_by_label("Couldn't open your web browser").is_none());
}

#[test]
fn escape_closes_the_failed_browser_dialog() {
    let mut h = harness(|app| app.url_opener = Some(Box::new(|_| Err("no browser".to_owned()))));
    h.state_mut().execute("help.report_issue");
    h.run_steps(3);
    h.get_by_label("Couldn't open your web browser");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(h.state().failed_link.is_none());
}
