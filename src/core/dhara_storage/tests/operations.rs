use std::fs;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dhara_storage::{
    ContentKind, DirectoryDeleteOptions, DirectoryStorage, FileStorage, SearchScope,
    StorageChangeType, StorageEntry, StorageError, StorageWatchConfig, TransferOptions,
    WriteOptions, copy_directory_with_options, copy_file_with_options, move_file_with_options,
    read_file_to_string, write_file,
};
use tempfile::tempdir;

#[test]
fn write_and_read_file_roundtrip() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("notes").join("entry.txt");

    write_file(&path, b"hello from rust").unwrap();
    let content = read_file_to_string(&path).unwrap();

    assert_eq!(content, "hello from rust");
}

#[test]
fn copy_file_with_progress_reports_completion() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.bin");
    let destination = temp.path().join("destination.bin");
    fs::write(&source, vec![7u8; 32 * 1024]).unwrap();

    let updates = Arc::new(Mutex::new(Vec::new()));
    let reporter_updates = Arc::clone(&updates);
    let reporter = Arc::new(move |progress| {
        reporter_updates.lock().unwrap().push(progress);
    });

    copy_file_with_options(
        &source,
        &destination,
        TransferOptions {
            overwrite: false,
            buffer_size: Some(8 * 1024),
            progress: Some(reporter),
            cancellation_token: None,
        },
    )
    .unwrap();

    let updates = updates.lock().unwrap();
    assert!(!updates.is_empty());
    assert_eq!(updates.last().unwrap().bytes_transferred, 32 * 1024);
    assert_eq!(fs::read(&destination).unwrap().len(), 32 * 1024);
}

#[test]
fn move_file_overwrites_destination_when_requested() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.json");
    let destination = temp.path().join("destination.json");
    fs::write(&source, br#"{"name":"source"}"#).unwrap();
    fs::write(&destination, br#"{"name":"old"}"#).unwrap();

    move_file_with_options(
        &source,
        &destination,
        TransferOptions {
            overwrite: true,
            ..TransferOptions::default()
        },
    )
    .unwrap();

    assert!(!source.exists());
    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        r#"{"name":"source"}"#
    );
}

#[test]
fn copy_directory_with_progress_preserves_tree() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source");
    let nested = source.join("nested");
    let destination = temp.path().join("copied");
    fs::create_dir_all(&nested).unwrap();
    fs::write(source.join("root.txt"), b"root").unwrap();
    fs::write(nested.join("child.txt"), b"child").unwrap();

    let updates = Arc::new(Mutex::new(Vec::new()));
    let reporter_updates = Arc::clone(&updates);
    let reporter = Arc::new(move |progress| {
        reporter_updates.lock().unwrap().push(progress);
    });

    copy_directory_with_options(
        &source,
        &destination,
        TransferOptions {
            overwrite: false,
            buffer_size: Some(4 * 1024),
            progress: Some(reporter),
            cancellation_token: None,
        },
    )
    .unwrap();

    assert_eq!(
        fs::read_to_string(destination.join("root.txt")).unwrap(),
        "root"
    );
    assert_eq!(
        fs::read_to_string(destination.join("nested").join("child.txt")).unwrap(),
        "child"
    );
    assert!(updates.lock().unwrap().last().unwrap().bytes_transferred >= 9);
}

#[test]
fn file_storage_wraps_operations_without_eager_analysis() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("story.txt");
    let file = FileStorage::new(&path).unwrap();

    file.write_string("story body").unwrap();
    let info = file.info_with_analysis().unwrap();

    assert_eq!(info.content_kind().unwrap(), ContentKind::Text);
    assert_eq!(info.mime_type().unwrap(), Some("text/plain"));
}

#[test]
fn file_storage_rename_returns_new_handle() {
    let temp = tempdir().unwrap();
    let original = temp.path().join("draft.txt");
    fs::write(&original, b"draft").unwrap();
    let file = FileStorage::from_existing(&original).unwrap();

    let renamed = file.rename("final.txt").unwrap();

    assert_eq!(renamed.name(), Some("final.txt"));
    assert!(renamed.path().exists());
    assert!(!original.exists());
}

#[test]
fn directory_storage_delete_can_be_non_recursive() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("folder");
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("child.txt"), b"child").unwrap();
    let directory = DirectoryStorage::from_existing(&path).unwrap();

    let err = directory
        .delete_with_options(DirectoryDeleteOptions {
            recursive: false,
            cancellation_token: None,
        })
        .unwrap_err();

    assert!(matches!(err, StorageError::Io { .. }));
    assert!(path.exists());
}

#[test]
fn storage_entry_resolves_existing_paths() {
    let temp = tempdir().unwrap();
    let file_path = temp.path().join("asset.txt");
    let dir_path = temp.path().join("folder");
    fs::write(&file_path, b"asset").unwrap();
    fs::create_dir_all(&dir_path).unwrap();

    assert!(matches!(
        StorageEntry::from_existing(&file_path).unwrap(),
        StorageEntry::File(_)
    ));
    assert!(matches!(
        StorageEntry::from_existing(&dir_path).unwrap(),
        StorageEntry::Directory(_)
    ));
}

#[test]
fn write_from_reader_supports_progress_reporting() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("stream.bin");
    let updates = Arc::new(Mutex::new(Vec::new()));
    let reporter_updates = Arc::clone(&updates);
    let reporter = Arc::new(move |progress| {
        reporter_updates.lock().unwrap().push(progress);
    });
    let mut cursor = Cursor::new(vec![3u8; 10 * 1024]);

    FileStorage::new(&path)
        .unwrap()
        .write_from_reader(
            &mut cursor,
            WriteOptions {
                progress: Some(reporter),
                ..WriteOptions::default()
            },
        )
        .unwrap();

    assert_eq!(fs::read(&path).unwrap().len(), 10 * 1024);
    assert_eq!(
        updates.lock().unwrap().last().unwrap().bytes_transferred,
        10 * 1024
    );
    assert!(
        updates
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .total_bytes
            .is_none()
    );
}

#[test]
fn directory_storage_can_enumerate_and_resolve_children() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("root");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.txt"), b"a").unwrap();
    fs::write(root.join("nested").join("b.log"), b"b").unwrap();
    let directory = DirectoryStorage::from_existing(&root).unwrap();

    let top_files = directory.files().unwrap();
    let recursive_files = directory
        .files_matching("*", SearchScope::AllDirectories)
        .unwrap();
    let nested = directory.get_directory("nested").unwrap();
    let nested_file = directory.get_file("nested/b.log").unwrap();

    assert_eq!(top_files.len(), 1);
    assert_eq!(recursive_files.len(), 2);
    assert_eq!(nested.name(), Some("nested"));
    assert_eq!(nested_file.name(), Some("b.log"));
}

#[test]
fn directory_storage_matching_supports_globs() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("root");
    fs::create_dir_all(root.join("logs")).unwrap();
    fs::write(root.join("one.txt"), b"1").unwrap();
    fs::write(root.join("logs").join("two.log"), b"2").unwrap();
    fs::write(root.join("logs").join("three.txt"), b"3").unwrap();
    let directory = DirectoryStorage::from_existing(&root).unwrap();

    let txt_files = directory
        .files_matching("*.txt", SearchScope::AllDirectories)
        .unwrap();
    let log_dirs = directory
        .directories_matching("log*", SearchScope::TopDirectoryOnly)
        .unwrap();

    assert_eq!(txt_files.len(), 2);
    assert_eq!(log_dirs.len(), 1);
}

#[test]
fn directory_storage_rejects_escaping_relative_paths() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("root");
    fs::create_dir_all(&root).unwrap();
    let directory = DirectoryStorage::from_existing(&root).unwrap();

    let err = directory.get_file("../outside.txt").unwrap_err();
    assert!(matches!(err, StorageError::PathConflict { .. }));
}

#[test]
fn directory_watch_reports_created_files() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("watched");
    fs::create_dir_all(&root).unwrap();
    let directory = DirectoryStorage::from_existing(&root).unwrap();
    let watcher = directory
        .watch(StorageWatchConfig {
            debounce_window: Duration::from_millis(100),
            ..StorageWatchConfig::default()
        })
        .unwrap();

    let created_file = root.join("created.txt");
    fs::write(&created_file, b"created").unwrap();

    let event = watcher
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .expect("expected a watcher event");

    assert_eq!(event.path, created_file);
    assert!(matches!(
        event.change_type,
        StorageChangeType::Created | StorageChangeType::Modified
    ));
}

#[cfg(feature = "async-tokio")]
#[tokio::test]
async fn async_file_operations_roundtrip() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("async.txt");
    let storage = FileStorage::new(&path).unwrap();

    storage
        .write_async("async content".as_bytes(), WriteOptions::default())
        .await
        .unwrap();
    let content = storage.read_to_string_async().await.unwrap();

    assert_eq!(content, "async content");
}

#[cfg(feature = "async-tokio")]
#[tokio::test]
async fn async_directory_copy_roundtrip() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("nested").join("value.txt"), b"value").unwrap();

    let storage = DirectoryStorage::from_existing(&source).unwrap();
    let copied = storage
        .copy_to_async(&destination, TransferOptions::default())
        .await
        .unwrap();

    assert_eq!(
        fs::read_to_string(copied.path().join("nested").join("value.txt")).unwrap(),
        "value"
    );
}
