use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Ensure `build_path/media` resolves to the durable media store at
/// `media_path`, so the generated static site is self-contained while the
/// media itself lives outside the regeneratable `build/` directory.
///
/// - Creates `media_path` (and `build_path`) if missing.
/// - Absent `build_path/media` → creates an absolute symlink to `media_path`.
/// - Already a symlink → re-points it only if it targets the wrong place.
/// - Legacy real directory → migrates its contents into `media_path` (when
///   `media_path` does not already exist, or exists but is empty) and replaces
///   it with the symlink. If BOTH the legacy dir and `media_path` hold media,
///   returns an error rather than risk clobbering.
/// - If symlink creation is unsupported/fails, copies `media_path` into
///   `build_path/media` and warns (non-fatal).
pub fn ensure_build_media(build_path: &Path, media_path: &Path) -> Result<()> {
    fs::create_dir_all(build_path)?;
    let link = build_path.join("media");
    let media_target = absolutize(media_path)?;
    let media_existed = media_path.exists();

    match fs::symlink_metadata(&link) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fs::create_dir_all(media_path)?;
            // Re-point only if it targets somewhere other than our media store.
            if fs::read_link(&link).ok().as_deref() != Some(media_target.as_path()) {
                fs::remove_file(&link)?; // removes the symlink entry, not the target
                symlink_dir(&media_target, &link)?;
            }
            return Ok(());
        }
        Ok(meta) if meta.file_type().is_dir() => {
            let legacy_nonempty = dir_has_entries(&link)?;
            let media_nonempty = media_existed && dir_has_entries(media_path)?;
            if legacy_nonempty && media_nonempty {
                return Err(anyhow!(
                    "both {link:?} (legacy media directory) and {media_path:?} contain \
                     media; refusing to migrate automatically. Merge them into \
                     {media_path:?}, remove {link:?}, then re-run."
                ));
            }
            if legacy_nonempty {
                // Promote the legacy dir to become the media store. An empty
                // media_path (if present) is treated as absent: remove it so the
                // rename lands cleanly and no legacy content is ever dropped.
                if media_existed {
                    fs::remove_dir_all(media_path)?; // safe: empty (media_nonempty == false here)
                }
                if let Some(parent) = media_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                move_dir(&link, media_path)?;
            } else {
                // Legacy dir is empty: drop it; media_path (if any) is authoritative.
                fs::remove_dir_all(&link)?;
            }
        }
        Ok(_) => {
            return Err(anyhow!(
                "{link:?} exists but is neither a directory nor a symlink; \
                 remove it and re-run."
            ));
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }

    fs::create_dir_all(media_path)?;
    if let Err(e) = symlink_dir(&media_target, &link) {
        warn!("could not symlink {link:?} -> {media_target:?} ({e}); copying media instead");
        // Note: this copy fallback is not idempotent across repeated builds on
        // symlink-less platforms (e.g. Windows without the symlink privilege) —
        // a subsequent build will see `link` as a populated legacy dir and may
        // hit the "refusing to migrate" ambiguity error above.
        fs::create_dir_all(&link)?;
        copy_dir_contents(media_path, &link)?;
    }
    Ok(())
}

fn absolutize(p: &Path) -> Result<PathBuf> {
    if p.is_absolute() {
        Ok(p.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(p))
    }
}

fn dir_has_entries(p: &Path) -> Result<bool> {
    Ok(fs::read_dir(p)?.next().is_some())
}

fn copy_dir_contents(from: &Path, to: &Path) -> Result<()> {
    let opts = fs_extra::dir::CopyOptions {
        overwrite: true,
        content_only: true,
        ..Default::default()
    };
    fs_extra::dir::copy(from, to, &opts)?;
    Ok(())
}

/// Move a directory, falling back to copy-then-remove when a plain rename is
/// not possible (e.g. `EXDEV` across filesystems).
fn move_dir(from: &Path, to: &Path) -> Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        // A plain rename only fails to cross filesystems with `CrossesDevices`
        // (EXDEV); fall back to copy-then-remove for that case alone. Any other
        // error (permissions, unexpected state) is real — surface it rather than
        // masking it behind a copy.
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => copy_dir_across(from, to),
        Err(e) => Err(e.into()),
    }
}

fn copy_dir_across(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir_all(to)?;
    let opts = fs_extra::dir::CopyOptions {
        overwrite: true,
        content_only: true,
        ..Default::default()
    };
    fs_extra::dir::copy(from, to, &opts)?;
    fs::remove_dir_all(from)?;
    Ok(())
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh, unique temp dir per test; removed first so reruns are clean.
    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fossilizer-media-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_symlink_when_build_media_absent() {
        let root = test_dir("create");
        let build = root.join("build");
        let media = root.join("media");
        fs::create_dir_all(&media).unwrap();
        fs::write(media.join("photo.jpg"), b"bytes").unwrap();

        ensure_build_media(&build, &media).unwrap();

        let link = build.join("media");
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        // Readable through the link.
        assert_eq!(fs::read(link.join("photo.jpg")).unwrap(), b"bytes");
    }

    #[test]
    fn is_idempotent_on_existing_symlink() {
        let root = test_dir("idempotent");
        let build = root.join("build");
        let media = root.join("media");

        ensure_build_media(&build, &media).unwrap();
        // Second call must not error or replace the media store.
        fs::write(media.join("keep.txt"), b"x").unwrap();
        ensure_build_media(&build, &media).unwrap();

        assert!(build.join("media").join("keep.txt").exists());
    }

    #[test]
    fn migrates_legacy_real_dir_when_media_absent() {
        let root = test_dir("migrate");
        let build = root.join("build");
        let media = root.join("media"); // does NOT exist yet
                                        // Legacy layout: real build/media dir with a file.
        fs::create_dir_all(build.join("media")).unwrap();
        fs::write(build.join("media").join("old.png"), b"legacy").unwrap();

        ensure_build_media(&build, &media).unwrap();

        // File now lives in the media store...
        assert_eq!(fs::read(media.join("old.png")).unwrap(), b"legacy");
        // ...and build/media is now a symlink to it.
        assert!(fs::symlink_metadata(build.join("media"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::read(build.join("media").join("old.png")).unwrap(),
            b"legacy"
        );
    }

    #[test]
    fn migrates_legacy_dir_when_media_exists_but_empty() {
        let root = test_dir("migrate-empty-media");
        let build = root.join("build");
        let media = root.join("media");
        // Legacy real dir WITH content...
        fs::create_dir_all(build.join("media")).unwrap();
        fs::write(build.join("media").join("old.png"), b"legacy").unwrap();
        // ...and media_path exists but is EMPTY.
        fs::create_dir_all(&media).unwrap();

        ensure_build_media(&build, &media).unwrap();

        // Legacy content migrated, nothing lost.
        assert_eq!(fs::read(media.join("old.png")).unwrap(), b"legacy");
        assert!(fs::symlink_metadata(build.join("media"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::read(build.join("media").join("old.png")).unwrap(),
            b"legacy"
        );
    }

    #[test]
    fn repoints_symlink_when_target_is_wrong() {
        let root = test_dir("repoint");
        let build = root.join("build");
        let right = root.join("media");
        let wrong = root.join("other-media");
        fs::create_dir_all(&right).unwrap();
        fs::write(right.join("r.txt"), b"right").unwrap();
        fs::create_dir_all(&wrong).unwrap();
        fs::create_dir_all(&build).unwrap();
        // Pre-existing symlink pointing at the WRONG place.
        super::symlink_dir(&wrong, &build.join("media")).unwrap();

        ensure_build_media(&build, &right).unwrap();

        // Now points at the right store and serves its content.
        assert_eq!(fs::read_link(build.join("media")).unwrap(), right);
        assert_eq!(
            fs::read(build.join("media").join("r.txt")).unwrap(),
            b"right"
        );
    }

    #[test]
    fn errors_when_both_legacy_dir_and_media_are_populated() {
        let root = test_dir("ambiguous");
        let build = root.join("build");
        let media = root.join("media");
        fs::create_dir_all(build.join("media")).unwrap();
        fs::write(build.join("media").join("a.png"), b"a").unwrap();
        fs::create_dir_all(&media).unwrap();
        fs::write(media.join("b.png"), b"b").unwrap();

        let result = ensure_build_media(&build, &media);
        assert!(result.is_err(), "must refuse to migrate ambiguous state");
        // Nothing destroyed.
        assert!(build.join("media").join("a.png").exists());
        assert!(media.join("b.png").exists());
    }

    #[test]
    fn clean_build_does_not_delete_media_through_symlink() {
        let root = test_dir("clean-safety");
        let build = root.join("build");
        let media = root.join("media");
        fs::create_dir_all(&media).unwrap();
        fs::write(media.join("important.bin"), b"do-not-lose").unwrap();

        // Establish the symlink as build would.
        ensure_build_media(&build, &media).unwrap();
        assert!(build.join("media").join("important.bin").exists());

        // Run the real clean routine used by `build --clean`.
        crate::site_generator::setup_build_path(&build, true).unwrap();

        // The build dir was wiped, but the media store MUST survive.
        assert!(
            media.join("important.bin").exists(),
            "clean must remove the symlink entry, never follow it into the media store"
        );
    }

    #[test]
    fn copy_dir_across_moves_content_and_removes_source() {
        let root = test_dir("cross-fs-move");
        let from = root.join("from");
        let to = root.join("to");
        fs::create_dir_all(&from).unwrap();
        fs::write(from.join("a.bin"), b"data").unwrap();

        super::copy_dir_across(&from, &to).unwrap();

        assert_eq!(fs::read(to.join("a.bin")).unwrap(), b"data");
        assert!(
            !from.exists(),
            "source must be removed after the cross-fs move"
        );
    }

    #[test]
    fn move_dir_surfaces_non_crossdevice_errors_without_copy_fallback() {
        let root = test_dir("move-dir-error");
        let from = root.join("does-not-exist");
        let to = root.join("to");

        // rename fails with NotFound (not CrossesDevices) — the error must be
        // surfaced, not masked by silently falling back to a copy.
        let result = super::move_dir(&from, &to);

        assert!(result.is_err());
        assert!(
            !to.exists(),
            "must not create the destination via copy fallback on a non-cross-device error"
        );
    }
}
