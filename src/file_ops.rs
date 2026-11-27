//! Local file operations shared by commands and the Files panel.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy)]
pub enum Operation {
    Rename,
    Copy,
    Move,
    Mkdir,
    New,
    Trash,
    Reveal,
}

fn absolute(path: &Path) -> Result<&Path, String> {
    path.is_absolute()
        .then_some(path)
        .ok_or_else(|| "path must be absolute".into())
}

fn collision(path: &Path) -> String {
    format!(
        "{} already exists",
        path.file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
    )
}

fn io_error(path: &Path, error: io::Error) -> String {
    if error.kind() == io::ErrorKind::AlreadyExists {
        collision(path)
    } else {
        format!("{}: {error}", path.display())
    }
}

fn vacant(path: &Path) -> Result<(), String> {
    match path.symlink_metadata() {
        Ok(_) => Err(collision(path)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(path, e)),
    }
}

/// Choose an available sibling name, preserving the final extension.
pub fn free_name(dir: &Path, name: &str) -> PathBuf {
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map_or(String::new(), |e| format!(".{e}"));
    let mut candidate = dir.join(name);
    for n in 1.. {
        if candidate.symlink_metadata().is_err() {
            break;
        }
        let suffix = if n == 1 {
            " copy".into()
        } else {
            format!(" copy {n}")
        };
        candidate = dir.join(format!("{stem}{suffix}{ext}"));
    }
    candidate
}

/// Resolve either an explicit destination or a paste folder.
pub fn destination(from: &Path, to: Option<&Path>, into: Option<&Path>) -> Result<PathBuf, String> {
    absolute(from)?;
    match (to, into) {
        (Some(to), None) => Ok(absolute(to)?.into()),
        (None, Some(into)) => {
            absolute(into)?;
            let name = from
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("source must have a file name")?;
            Ok(free_name(into, name))
        }
        _ => Err("provide exactly one of to or into".into()),
    }
}

pub fn rename(from: &Path, to: &Path) -> Result<(), String> {
    absolute(from)?;
    absolute(to)?;
    vacant(to)?;
    rename_exclusive(from, to).map_err(|e| io_error(to, e))
}

// shortcut: the vacancy check and the rename are two steps, so a file created in between is
// replaced; use a native no-replace rename if agents start racing the tree.
fn rename_exclusive(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

fn copy_entry(from: &Path, to: &Path) -> io::Result<()> {
    let meta = from.symlink_metadata()?;
    if meta.is_symlink() {
        let link = fs::read_link(from)?;
        #[cfg(unix)]
        return std::os::unix::fs::symlink(link, to);
        #[cfg(windows)]
        {
            use std::os::windows::fs::{FileTypeExt, symlink_dir, symlink_file};
            return if meta.file_type().is_symlink_dir() {
                symlink_dir(link, to)
            } else {
                symlink_file(link, to)
            };
        }
    }
    if meta.is_dir() {
        fs::create_dir(to)?;
        let result = (|| {
            for entry in fs::read_dir(from)? {
                let entry = entry?;
                copy_entry(&entry.path(), &to.join(entry.file_name()))?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(to);
        }
        return result;
    }
    if !meta.is_file() {
        return Err(io::Error::other("unsupported file type"));
    }
    let mut input = File::open(from)?;
    let mut output = File::create_new(to)?;
    let result =
        io::copy(&mut input, &mut output).and_then(|_| output.set_permissions(meta.permissions()));
    drop(output);
    if result.is_err() {
        let _ = fs::remove_file(to);
    }
    result.map(|_| ())
}

pub fn copy(from: &Path, to: &Path) -> Result<(), String> {
    absolute(from)?;
    absolute(to)?;
    vacant(to)?;
    if from
        .symlink_metadata()
        .map_err(|e| io_error(from, e))?
        .is_dir()
    {
        let source = from.canonicalize().map_err(|e| io_error(from, e))?;
        let parent = to.parent().ok_or("destination must have a parent")?;
        if parent
            .canonicalize()
            .map_err(|e| io_error(parent, e))?
            .starts_with(source)
        {
            return Err("cannot copy a folder into itself".into());
        }
    }
    copy_entry(from, to).map_err(|e| io_error(to, e))
}

pub fn move_entry(from: &Path, to: &Path) -> Result<(), String> {
    absolute(from)?;
    absolute(to)?;
    vacant(to)?;
    match rename_exclusive(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            copy(from, to)?;
            let meta = from.symlink_metadata().map_err(|e| io_error(from, e))?;
            #[cfg(windows)]
            use std::os::windows::fs::FileTypeExt;
            #[cfg(windows)]
            if meta.file_type().is_symlink_dir() {
                return fs::remove_dir(from).map_err(|e| io_error(from, e));
            }
            let result = if meta.is_dir() {
                fs::remove_dir_all(from)
            } else {
                fs::remove_file(from)
            };
            result.map_err(|e| io_error(from, e))
        }
        Err(e) => Err(io_error(to, e)),
    }
}

pub fn mkdir(path: &Path) -> Result<(), String> {
    fs::create_dir(absolute(path)?).map_err(|e| io_error(path, e))
}

pub fn new_file(path: &Path) -> Result<(), String> {
    File::create_new(absolute(path)?)
        .map(|_| ())
        .map_err(|e| io_error(path, e))
}

/// Move an item to the system trash.
pub fn trash(path: &Path) -> Result<(), String> {
    let path = absolute(path)?;
    #[cfg(target_os = "macos")]
    return macos_trash(path);
    #[cfg(target_os = "linux")]
    return command_result(
        Command::new("gio").arg("trash").arg(path),
        Some("Install gio to move files to the trash"),
    );
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        Err("Moving to the trash is not supported here".into())
    }
}

#[cfg(target_os = "macos")]
fn macos_trash(path: &Path) -> Result<(), String> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    let path = path.to_str().ok_or("path must be valid UTF-8")?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    NSFileManager::defaultManager()
        .trashItemAtURL_resultingItemURL_error(&url, None)
        .map_err(|e| e.localizedDescription().to_string())
}

/// Show an item's location in the system file browser.
pub fn reveal(path: &Path) -> Result<(), String> {
    let path = absolute(path)?;
    if cfg!(target_os = "macos") {
        command_result(Command::new("open").arg("-R").arg(path), None)
    } else if cfg!(target_os = "linux") {
        command_result(
            Command::new("xdg-open").arg(path.parent().unwrap_or(path)),
            None,
        )
    } else {
        Err("Showing a file in its folder is not supported here".into())
    }
}

fn command_result(command: &mut Command, missing: Option<&str>) -> Result<(), String> {
    let output = command.output().map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound
            && let Some(message) = missing
        {
            message.into()
        } else {
            e.to_string()
        }
    })?;
    if output.status.success() {
        return Ok(());
    }
    let error = String::from_utf8_lossy(&output.stderr);
    let error = error.trim();
    Err(if error.is_empty() {
        format!("{:?} exited with {}", command.get_program(), output.status)
    } else {
        error.into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_ops_round_trip() {
        let root = std::env::temp_dir().join(format!("canopy-file-ops-{}", std::process::id()));
        mkdir(&root).unwrap();
        struct Temp(PathBuf);
        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _temp = Temp(root.clone());
        let src = root.join("src");
        mkdir(&src).unwrap();
        assert_eq!(mkdir(&src).unwrap_err(), "src already exists");
        let file = src.join("a.txt");
        new_file(&file).unwrap();
        assert_eq!(new_file(&file).unwrap_err(), "a.txt already exists");
        fs::write(&file, "hello").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("a.txt", src.join("link")).unwrap();
        assert_eq!(free_name(&src, "a.txt"), src.join("a copy.txt"));
        new_file(&src.join("a copy.txt")).unwrap();
        assert_eq!(free_name(&src, "a.txt"), src.join("a copy 2.txt"));
        assert_eq!(free_name(&src, "b"), src.join("b"));
        let duplicate = root.join("duplicate");
        copy(&src, &duplicate).unwrap();
        assert_eq!(
            fs::read_to_string(duplicate.join("a.txt")).unwrap(),
            "hello"
        );
        #[cfg(unix)]
        assert_eq!(
            fs::read_link(duplicate.join("link")).unwrap(),
            Path::new("a.txt")
        );
        assert!(copy(&src, &src.join("inside")).is_err());
        assert_eq!(
            copy(&src, &duplicate).unwrap_err(),
            "duplicate already exists"
        );
        assert_eq!(
            move_entry(&src, &duplicate).unwrap_err(),
            "duplicate already exists"
        );
        assert_eq!(
            rename(&src, &duplicate).unwrap_err(),
            "duplicate already exists"
        );
        let renamed = root.join("renamed");
        rename(&duplicate, &renamed).unwrap();
        move_entry(&renamed, &duplicate).unwrap();
        assert!(!renamed.exists());
        assert!(duplicate.join("a.txt").exists());
        assert_eq!(
            mkdir(Path::new("relative")).unwrap_err(),
            "path must be absolute"
        );
        assert!(destination(&src, Some(&duplicate), Some(&root)).is_err());
    }
}
