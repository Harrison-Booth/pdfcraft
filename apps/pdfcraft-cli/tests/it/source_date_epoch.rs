//! Exercise the environment through subprocesses: the test runner's clock and environment stay
//! untouched, and the assertions cover the dates in the files that CLI users actually receive.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use pdfcraft_cos::Document;

const BIN: &str = env!("CARGO_BIN_EXE_pdfcraft-cli");

struct Scratch(tempfile::TempDir);

impl Scratch {
    fn new(test: &str) -> Self {
        Self(tempfile::Builder::new().prefix(&format!("pdfcraft-cli-epoch-{test}-")).tempdir().unwrap())
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.path().join(name)
    }

    fn input(&self, name: &str) -> PathBuf {
        let path = self.path(name);
        // A contributor-original one-page PDF; its bytes and dates do not depend on the clock.
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (i, object) in [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 300] >>",
            "<< /Type /Page /Parent 2 0 R >>",
        ]
        .iter()
        .enumerate()
        {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", i + 1).as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        for offset in offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
        std::fs::write(&path, out).unwrap();
        path
    }
}

fn run(command: &mut Command, epoch: Option<&str>, output: &Path) -> Vec<u8> {
    // Remove any epoch inherited from the build machine, even in the unset control.
    command.env_remove("SOURCE_DATE_EPOCH");
    if let Some(epoch) = epoch {
        command.env("SOURCE_DATE_EPOCH", epoch);
    }
    let result = command.arg("--out").arg(output).output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    std::fs::read(output).unwrap()
}

fn mod_date(bytes: &[u8]) -> String {
    let doc = Document::open(Arc::new(bytes.to_vec())).unwrap();
    let info = doc.resolve(doc.trailer().get(b"Info").expect("saved file has /Info"));
    info.as_dict().and_then(|d| d.get(b"ModDate")).and_then(|o| o.as_string()).expect("saved file has /ModDate").to_text()
}

#[test]
fn combine_uses_the_epoch_and_repeats_identical_bytes() {
    let scratch = Scratch::new("combine");
    let (a, b) = (scratch.input("a.pdf"), scratch.input("b.pdf"));
    for (epoch, expected) in [("0", "D:19700101000000Z"), ("1700000000", "D:20231114221320Z")] {
        let convert = |name| run(Command::new(BIN).arg("combine").arg(&a).arg(&b), Some(epoch), &scratch.path(name));
        let first = convert("one.pdf");
        assert_eq!(mod_date(&first), expected);
        assert_eq!(first, convert("two.pdf"), "combine changed bytes under epoch {epoch}");
    }
}

#[test]
fn full_and_incremental_edit_use_the_epoch_and_repeat_identical_bytes() {
    let scratch = Scratch::new("edit");
    let input = scratch.input("in.pdf");
    let original = std::fs::read(&input).unwrap();
    for full in [false, true] {
        let edit = |name| {
            let mut command = Command::new(BIN);
            command.arg("edit").arg(&input).args(["--title", "Reproducible"]);
            if full {
                command.arg("--full");
            }
            run(&mut command, Some("1700000000"), &scratch.path(name))
        };
        let first = edit("one.pdf");
        assert_eq!(mod_date(&first), "D:20231114221320Z");
        assert_eq!(first.starts_with(&original), !full, "exercise the requested save mode");
        assert_eq!(first, edit("two.pdf"), "edit changed bytes (full={full})");
    }
}

#[test]
fn unset_or_invalid_epochs_keep_the_wall_clock() {
    let scratch = Scratch::new("fallback");
    let (a, b) = (scratch.input("a.pdf"), scratch.input("b.pdf"));
    for epoch in [None, Some(""), Some("yesterday"), Some("1.5"), Some("-1"), Some("9223372036854775808")] {
        let now = || SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let before = pdfcraft_cos::pdf_date(now());
        let bytes = run(Command::new(BIN).arg("combine").arg(&a).arg(&b), epoch, &scratch.path("out.pdf"));
        let after = pdfcraft_cos::pdf_date(now());
        let date = mod_date(&bytes);
        assert!(before <= date && date <= after, "{epoch:?} should use the wall clock: {date} outside {before}..={after}");
    }
}
