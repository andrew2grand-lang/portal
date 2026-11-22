use std::fs;
use std::io::ErrorKind;
use std::sync::{Arc, Barrier};

use portal_sys::fs::rename_noreplace;

#[test]
fn moves_a_file_when_the_target_is_free() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("a.txt");
    let to = dir.path().join("b.txt");
    fs::write(&from, b"hello").unwrap();

    rename_noreplace(&from, &to).unwrap();

    assert!(!from.exists());
    assert_eq!(fs::read(&to).unwrap(), b"hello");
}

#[test]
fn refuses_to_replace_an_existing_file_and_leaves_both_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("new.txt");
    let to = dir.path().join("old.txt");
    fs::write(&from, b"new").unwrap();
    fs::write(&to, b"old").unwrap();

    let err = rename_noreplace(&from, &to).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::AlreadyExists);
    assert_eq!(fs::read(&from).unwrap(), b"new");
    assert_eq!(fs::read(&to).unwrap(), b"old");
}

#[test]
fn moves_a_directory_with_its_contents() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("folder");
    fs::create_dir_all(from.join("inner")).unwrap();
    fs::write(from.join("inner/f.txt"), b"x").unwrap();
    let to = dir.path().join("moved");

    rename_noreplace(&from, &to).unwrap();

    assert_eq!(fs::read(to.join("inner/f.txt")).unwrap(), b"x");
}

#[test]
fn refuses_to_replace_an_existing_directory() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("a");
    let to = dir.path().join("b");
    fs::create_dir(&from).unwrap();
    fs::create_dir(&to).unwrap();

    let err = rename_noreplace(&from, &to).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::AlreadyExists);
    assert!(from.exists());
}

#[test]
fn exactly_one_of_many_racing_renames_wins() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.bin");
    let racers = 16;
    let barrier = Arc::new(Barrier::new(racers));
    let mut handles = Vec::new();
    for i in 0..racers {
        let src = dir.path().join(format!("src-{i}"));
        fs::write(&src, format!("{i}")).unwrap();
        let (barrier, target) = (barrier.clone(), target.clone());
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            rename_noreplace(&src, &target).map(|_| i)
        }));
    }
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    let winners: Vec<usize> = results.iter().filter_map(|r| r.as_ref().ok().copied()).collect();
    assert_eq!(winners.len(), 1, "exactly one rename may succeed: {results:?}");
    for r in results.iter().filter(|r| r.is_err()) {
        assert_eq!(r.as_ref().unwrap_err().kind(), ErrorKind::AlreadyExists);
    }
    assert_eq!(fs::read_to_string(&target).unwrap(), winners[0].to_string());
}
