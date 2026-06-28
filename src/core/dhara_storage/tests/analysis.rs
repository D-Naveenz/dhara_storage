use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Write};
use std::path::PathBuf;

use dhara_storage::{
    ContentKind, DirectoryInfo, FileInfo, StorageError, analyze_path, analyze_reader,
};
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn analyze_path_missing_file_returns_not_found() {
    let temp = tempdir().unwrap();
    let missing = temp.path().join("missing.bin");

    let err = analyze_path(&missing).unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

#[test]
fn analyze_path_empty_file_returns_unknown_report() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("empty.bin");
    File::create(&path).unwrap();

    let report = analyze_path(&path).unwrap();
    assert!(report.is_empty());
    assert_eq!(report.content_kind, ContentKind::Unknown);
    assert_eq!(report.top_mime_type, None);
}

#[test]
fn analyze_reader_detects_png_signature() {
    let png_bytes = [
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D',
        b'R',
    ];
    let cursor = Cursor::new(png_bytes);

    let report = analyze_reader(cursor, Some(std::path::Path::new("image.png"))).unwrap();

    assert!(!report.matches.is_empty());
    assert_eq!(report.top_detected_extension.as_deref(), Some("png"));
    assert_eq!(report.top_mime_type.as_deref(), Some("image/png"));
    assert_eq!(report.content_kind, ContentKind::Binary);
}

#[test]
fn analyze_reader_detects_zip_signature() {
    let zip_bytes = [0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00];
    let cursor = Cursor::new(zip_bytes);

    let report = analyze_reader(cursor, Some(std::path::Path::new("archive.zip"))).unwrap();

    assert!(!report.matches.is_empty());
    assert!(
        report
            .matches
            .iter()
            .flat_map(|item| item.extensions.iter())
            .any(|ext| ext.eq_ignore_ascii_case("zip"))
    );
}

#[test]
fn analyze_reader_detects_pdf_signature() {
    let report = analyze_path(fixture_path("sample-2.pdf")).unwrap();

    assert_eq!(report.top_mime_type.as_deref(), Some("application/pdf"));
    assert_eq!(report.top_detected_extension.as_deref(), Some("pdf"));
    assert_eq!(report.content_kind, ContentKind::Binary);
}

#[test]
fn analyze_path_detects_real_mp4_fixture() {
    let report = analyze_path(fixture_path("sample-4.mp4")).unwrap();

    assert_eq!(report.top_detected_extension.as_deref(), Some("mp4"));
    assert_eq!(report.content_kind, ContentKind::Binary);
}

#[test]
fn analyze_reader_uses_text_fallback_for_plain_text() {
    let cursor = Cursor::new(b"Hello from Dhara.Storage in Rust.\n".to_vec());

    let report = analyze_reader(cursor, Some(std::path::Path::new("note.txt"))).unwrap();

    assert_eq!(report.content_kind, ContentKind::Text);
    assert_eq!(report.top_mime_type.as_deref(), Some("text/plain"));
    assert_eq!(report.top_detected_extension.as_deref(), Some("txt"));
}

#[test]
fn analyze_reader_detects_utf8_bom_text() {
    let cursor = Cursor::new(vec![0xEF, 0xBB, 0xBF, b'H', b'i', b'!']);

    let report = analyze_reader(cursor, Some(std::path::Path::new("bom.txt"))).unwrap();

    assert_eq!(report.content_kind, ContentKind::Text);
    assert_eq!(report.top_mime_type.as_deref(), Some("text/plain"));
}

#[test]
fn analyze_reader_normalizes_source_extension() {
    let cursor = Cursor::new(b"plain text".to_vec());

    let report = analyze_reader(cursor, Some(std::path::Path::new("README.TXT"))).unwrap();

    assert_eq!(report.source_extension.as_deref(), Some("txt"));
}

#[test]
fn analyze_reader_uses_binary_fallback_for_unknown_bytes() {
    let cursor = Cursor::new(vec![0x00, 0xFF, 0x7A, 0x3C, 0x5D, 0xA1, 0x42, 0x99]);

    let report = analyze_reader(cursor, Some(std::path::Path::new("mystery.bin"))).unwrap();

    assert_eq!(report.content_kind, ContentKind::Binary);
    assert_eq!(
        report.top_mime_type.as_deref(),
        Some("application/octet-stream")
    );
}

#[test]
fn ranked_matches_are_sorted_and_confidence_sums_to_roughly_hundred() {
    let cursor = Cursor::new([
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D',
        b'R',
    ]);

    let report = analyze_reader(cursor, Some(std::path::Path::new("image.png"))).unwrap();

    for pair in report.matches.windows(2) {
        assert!(pair[0].score >= pair[1].score);
    }

    let total_confidence = report
        .matches
        .iter()
        .map(|item| item.confidence)
        .sum::<f64>();
    assert!((total_confidence - 100.0).abs() < 0.01);
}

#[cfg(windows)]
#[test]
fn analyze_path_can_read_file_while_another_handle_is_open() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("shared.txt");
    fs::write(&path, b"shared read").unwrap();

    let mut open_handle = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    writeln!(open_handle).unwrap();

    let report = analyze_path(&path).unwrap();
    assert_eq!(report.content_kind, ContentKind::Text);
}

#[test]
fn file_info_from_path_exposes_rust_native_metadata() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("sample.txt");
    fs::write(&path, b"hello file info").unwrap();

    let info = FileInfo::from_path(&path).unwrap();

    assert_eq!(info.filename_extension(), Some("txt"));
    assert_eq!(info.size(), 15);
    assert_eq!(info.display_name(), "sample");
    assert_eq!(info.mime_type().unwrap(), Some("text/plain"));
    assert_eq!(info.content_kind().unwrap(), ContentKind::Text);
}

#[test]
fn file_info_from_path_rejects_directories() {
    let temp = tempdir().unwrap();

    let err = FileInfo::from_path(temp.path()).unwrap_err();
    assert!(matches!(err, StorageError::NotAFile { .. }));
}

#[test]
fn file_info_analysis_is_lazy() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("lazy.txt");
    fs::write(&path, b"lazy analysis").unwrap();

    let info = FileInfo::from_path(&path).unwrap();
    fs::remove_file(&path).unwrap();

    let err = info.analysis().unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

#[test]
fn file_info_preloaded_analysis_survives_file_removal() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("preloaded.txt");
    fs::write(&path, b"preloaded analysis").unwrap();

    let info = FileInfo::from_path_with_analysis(&path).unwrap();
    fs::remove_file(&path).unwrap();

    assert_eq!(info.mime_type().unwrap(), Some("text/plain"));
    assert_eq!(info.content_kind().unwrap(), ContentKind::Text);
}

#[test]
fn file_info_type_name_prefers_loaded_analysis() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("typed.txt");
    fs::write(&path, b"typed").unwrap();

    let info = FileInfo::from_path_with_analysis(&path).unwrap();

    assert_eq!(info.type_name(), "Plain Text");
}

#[test]
fn directory_info_summary_is_lazy() {
    let temp = tempdir().unwrap();
    let nested = temp.path().join("nested");
    fs::create_dir(&nested).unwrap();

    let info = DirectoryInfo::from_path(temp.path()).unwrap();
    fs::write(temp.path().join("late.txt"), b"late").unwrap();
    fs::write(nested.join("deep.txt"), b"deep").unwrap();

    let summary = info.summary().unwrap();
    assert_eq!(summary.file_count, 2);
    assert_eq!(summary.directory_count, 1);
}

#[test]
fn directory_info_preloaded_summary_is_cached() {
    let temp = tempdir().unwrap();
    let nested = temp.path().join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(temp.path().join("existing.txt"), b"existing").unwrap();

    let info = DirectoryInfo::from_path_with_summary(temp.path()).unwrap();
    fs::write(nested.join("later.txt"), b"later").unwrap();

    let summary = info.summary().unwrap();
    assert_eq!(summary.file_count, 1);
    assert_eq!(summary.directory_count, 1);
}

#[test]
fn directory_info_defaults_type_name_when_no_shell_data_is_needed() {
    let temp = tempdir().unwrap();

    let info = DirectoryInfo::from_path(temp.path()).unwrap();
    assert!(!info.type_name().is_empty());
}
