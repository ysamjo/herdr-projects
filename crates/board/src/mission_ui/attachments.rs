//! Read an explicitly requested macOS clipboard image and keep it outside the project for the interactive agent.
use crate::err::Res;
use std::path::{Path, PathBuf};

const CLIPBOARD: &str = r#"
ObjC.import('AppKit');
function run() {
    const board = $.NSPasteboard.generalPasteboard;
    let data = board.dataForType($.NSPasteboardTypePNG);
    if (!data || data.isNil()) {
        const tiff = board.dataForType($.NSPasteboardTypeTIFF);
        if (!tiff || tiff.isNil()) throw Error('Kein Bild in der Mac-Zwischenablage. Zuerst ein Bild kopieren.');
        const bitmap = $.NSBitmapImageRep.imageRepWithData(tiff);
        if (!bitmap || bitmap.isNil()) throw Error('Das Bildformat kann nicht gelesen werden.');
        data = bitmap.representationUsingTypeProperties($.NSBitmapImageFileTypePNG, $({}));
    }
    if (!data || data.isNil() || data.length === 0) throw Error('Das Bild ist leer.');
    if (data.length > 20 * 1024 * 1024) throw Error('Das Bild ist größer als 20 MB.');
    return ObjC.unwrap(data.base64EncodedStringWithOptions(0));
}
"#;

pub fn clipboard(folder: &Path, socket: &str) -> Res<PathBuf> {
    if !cfg!(target_os = "macos") { crate::fail!("Bild einfügen benötigt die lokale Mac-Zwischenablage. Alternativ den Bildpfad ins Taskfeld einfügen.") }
    let co = crate::mission_control::project_coordinator(folder, socket)?;
    let output = std::process::Command::new("/usr/bin/osascript").args(["-l", "JavaScript", "-e", CLIPBOARD]).output()?;
    if !output.status.success() {
        crate::fail!("Bild konnte nicht eingefügt werden: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let bytes = crate::util::unbase64(String::from_utf8_lossy(&output.stdout).trim())
        .ok_or_else(|| crate::err::Error::new("Das Bild aus der Zwischenablage konnte nicht gelesen werden."))?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") || bytes.len() > 20 * 1024 * 1024 { crate::fail!("Kein gültiges PNG-Bild bis 20 MB erhalten.") }
    let dir = crate::coordinator::state::dir_of(&co).join("attachments");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("bild-{}.png", crate::util::random_hex(16)));
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(&path)?.write_all(&bytes)?;
    Ok(path)
}

pub fn insert(s: &mut super::State, folder: PathBuf, path: PathBuf) {
    let reference = format!("\nBildanhang: {}\nLies diese Bilddatei für diesen Auftrag.\n", path.display());
    if s.folder().map(|p| p.path == folder).unwrap_or(false) {
        s.draft.insert(&reference); s.save_draft();
    } else {
        s.drafts.entry(folder.display().to_string()).or_default().push_str(&reference);
    }
    s.error.clear(); s.notice = "Bild angehängt · Auftrag ergänzen und senden".into();
}
