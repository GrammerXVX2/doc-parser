use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::pipeline::{PipelineContext, run_pipeline};
use uuid::Uuid;
use zip::write::SimpleFileOptions;

fn pipeline_context() -> PipelineContext {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pipeline = load_pipeline_config(&root.join("configs/pipeline.config.jsonc"))
        .expect("pipeline config should load");
    let routing = load_format_routing_config(&root.join("configs/format_routing.config.jsonc"))
        .expect("routing config should load");
    PipelineContext::new(pipeline, routing)
}

fn temp_xlsx_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{}_{}_{}.xlsx", name, std::process::id(), Uuid::new_v4()))
}

fn write_xlsx(path: &std::path::Path, sheet_xml: &str) {
    let file = File::create(path).expect("create xlsx");
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default();

    let entries = vec![
        (
            "[Content_Types].xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\n  <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\n  <Default Extension=\"xml\" ContentType=\"application/xml\"/>\n  <Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\n  <Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\n</Types>",
        ),
        (
            "_rels/.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n  <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/>\n</Relationships>",
        ),
        (
            "xl/workbook.xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\n  <sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"rId1\"/></sheets>\n</workbook>",
        ),
        (
            "xl/_rels/workbook.xml.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n  <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/>\n</Relationships>",
        ),
        ("xl/worksheets/sheet1.xml", sheet_xml),
    ];

    for (name, content) in entries {
        zip.start_file(name, opts).expect("start file");
        zip.write_all(content.as_bytes()).expect("write bytes");
    }

    zip.finish().expect("finish xlsx");
}

#[test]
fn docx_russian_text_language_ru() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("testdata/office/sample_ru.docx");
    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("pipeline");

    assert_eq!(model.document_profile.language_info.primary.as_deref(), Some("ru"));
}

#[test]
fn xlsx_mixed_ru_en_has_language_metadata() {
    let path = temp_xlsx_path("mixed_lang");
    let sheet = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>\n  <row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Metric</t></is></c><c r=\"B1\" t=\"inlineStr\"><is><t>Значение</t></is></c></row>\n  <row r=\"2\"><c r=\"A2\" t=\"inlineStr\"><is><t>Revenue</t></is></c><c r=\"B2\" t=\"inlineStr\"><is><t>Доход</t></is></c></row>\n</sheetData></worksheet>";
    write_xlsx(&path, sheet);

    let (_, model) = run_pipeline(&path, &pipeline_context()).expect("pipeline");
    assert!(model.document_profile.language_info.primary.is_some());
    assert!(!model.document_profile.languages.is_empty());

    let _ = std::fs::remove_file(path);
}

#[test]
fn xlsx_numbers_only_fallback_ru() {
    let path = temp_xlsx_path("numbers_only");
    let sheet = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>\n  <row r=\"1\"><c r=\"A1\"><v>1</v></c><c r=\"B1\"><v>2</v></c></row>\n  <row r=\"2\"><c r=\"A2\"><v>3</v></c><c r=\"B2\"><v>4</v></c></row>\n</sheetData></worksheet>";
    write_xlsx(&path, sheet);

    let (_, model) = run_pipeline(&path, &pipeline_context()).expect("pipeline");
    assert_eq!(model.document_profile.language_info.primary.as_deref(), Some("ru"));

    let _ = std::fs::remove_file(path);
}
