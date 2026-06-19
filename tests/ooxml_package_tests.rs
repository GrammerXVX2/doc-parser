use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use document_parser::office::OoxmlPackage;
use uuid::Uuid;
use zip::write::SimpleFileOptions;

fn temp_zip_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{}_{}_{}.zip", name, std::process::id(), Uuid::new_v4()))
}

fn write_zip(path: &std::path::Path, entries: Vec<(&str, Vec<u8>)>) {
    let file = File::create(path).expect("create temp zip");
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    for (name, bytes) in entries {
        zip.start_file(name, opts).expect("start zip entry");
        zip.write_all(&bytes).expect("write entry bytes");
    }
    zip.finish().expect("finish zip");
}

#[test]
fn open_valid_package_and_read_document_xml() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/office/sample_ru.docx");
    let pkg = OoxmlPackage::open(&path).expect("open valid docx package");

    let entries = pkg.list_entries();
    assert!(!entries.is_empty());

    let doc = pkg
        .read_text("word/document.xml")
        .expect("read document xml")
        .expect("document xml exists");
    assert!(doc.contains("w:document"));
}

#[test]
fn reject_path_traversal_entry() {
    let path = temp_zip_path("ooxml_path_traversal");
    write_zip(
        &path,
        vec![
            ("word/document.xml", b"<w:document/>".to_vec()),
            ("../evil.txt", b"boom".to_vec()),
        ],
    );

    let err = OoxmlPackage::open(&path).expect_err("must reject path traversal entry");
    let msg = format!("{err:#}");
    assert!(msg.contains("OOXML_PATH_TRAVERSAL_BLOCKED"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn reject_entry_size_limit() {
    let path = temp_zip_path("ooxml_entry_too_large");
    let large = vec![b'a'; 33 * 1024 * 1024];
    write_zip(&path, vec![("word/document.xml", large)]);

    let err = OoxmlPackage::open(&path).expect_err("must reject large entry");
    let msg = format!("{err:#}");
    assert!(msg.contains("OOXML_ENTRY_TOO_LARGE"));

    let _ = std::fs::remove_file(path);
}
