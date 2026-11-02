// Embeds nowbar.manifest: Windows 10/11 support (needed for layered child windows) and
// Per-Monitor v2 DPI awareness. Embeds the icon (assets/nowbar.ico). Builds the English table
// `i18n::tr` searches from i18n/en/LC_MESSAGES/nowbar-ui.po. The settings window's Slint UI
// compiles in its own crate (ui/), with the same .po bundled.
//
// Both are copied into OUT_DIR and linked from there: checkouts that share a target dir (the
// main one and its worktrees) share this script's output too, and a path into a checkout that
// is gone breaks every link after it.
use std::path::{Path, PathBuf};
use std::process::Command;

include!("src/po.rs");

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    println!("cargo:rerun-if-changed=nowbar.manifest");
    let manifest = out.join("nowbar.manifest");
    std::fs::copy(root.join("nowbar.manifest"), &manifest).expect("nowbar.manifest");
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest.display());
    embed_icon(&root.join("assets").join("nowbar.ico"), &root.join("assets").join("nowbar-nobg.ico"), &out);
    english_table(root, &out);
}

/// The English strings as a table sorted by their Turkish (OUT_DIR/en.rs, included by i18n).
fn english_table(root: &Path, out: &Path) {
    let po = root.join("i18n").join("en").join("LC_MESSAGES").join("nowbar-ui.po");
    println!("cargo:rerun-if-changed={}", po.display());
    let mut entries = parse_po(&std::fs::read_to_string(&po).unwrap_or_default());
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.dedup_by(|a, b| a.0 == b.0);
    let rows: String = entries.iter().map(|(tr, en)| format!("    ({tr:?}, {en:?}),\n")).collect();
    let table = format!("/// (Turkish, English), sorted by the Turkish.\nstatic EN: &[(&str, &str)] = &[\n{rows}];\n");
    std::fs::write(out.join("en.rs"), table).expect("en.rs");
}

/// Compiles a resource script with the Windows SDK's rc.exe and links it into every
/// exe; without the SDK the exes keep Windows' default icon (a warning, not an error).
/// Resource 1 = Main app logo (with background); Resource 2 = No-background logo (tray & taskbar).
fn embed_icon(source: &Path, nobg_source: &Path, out: &Path) {
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", nobg_source.display());
    let Some(rc) = find_rc() else {
        println!("cargo:warning=rc.exe (Windows SDK) not found: the exes keep the default icon");
        return;
    };
    let ico = out.join("nowbar.ico");
    std::fs::copy(source, &ico).expect("nowbar.ico");
    let nobg_ico = out.join("nowbar-nobg.ico");
    let _ = std::fs::copy(nobg_source, &nobg_ico);
    let script = out.join("nowbar.rc");
    let path = ico.display().to_string().replace('\\', "\\\\");
    let nobg_path = nobg_ico.display().to_string().replace('\\', "\\\\");
    let rc_body = if nobg_ico.exists() {
        format!(
            "1 ICON \"{path}\"\n2 ICON \"{nobg_path}\"\n\
1 VERSIONINFO\n\
FILEVERSION 1,0,0,0\n\
PRODUCTVERSION 1,0,0,0\n\
FILEFLAGSMASK 0x3fL\n\
FILEFLAGS 0x0L\n\
FILEOS 0x40004L\n\
FILETYPE 0x1L\n\
FILESUBTYPE 0x0L\n\
BEGIN\n\
    BLOCK \"StringFileInfo\"\n\
    BEGIN\n\
        BLOCK \"040904b0\"\n\
        BEGIN\n\
            VALUE \"CompanyName\", \"XDNOT Labs\"\n\
            VALUE \"FileDescription\", \"Swift Tab\"\n\
            VALUE \"FileVersion\", \"1.0.0.0\"\n\
            VALUE \"InternalName\", \"Swift Tab\"\n\
            VALUE \"OriginalFilename\", \"swifttab.exe\"\n\
            VALUE \"ProductName\", \"Swift Tab\"\n\
            VALUE \"ProductVersion\", \"1.0.0.0\"\n\
        END\n\
    END\n\
    BLOCK \"VarFileInfo\"\n\
    BEGIN\n\
        VALUE \"Translation\", 0x409, 1200\n\
    END\n\
END\n"
        )
    } else {
        format!("1 ICON \"{path}\"\n")
    };
    std::fs::write(&script, rc_body).expect("nowbar.rc");
    let res = out.join("nowbar.res");
    let built = Command::new(rc).arg("/nologo").arg("/fo").arg(&res).arg(&script).status().is_ok_and(|s| s.success());
    if built {
        println!("cargo:rustc-link-arg-bins={}", res.display());
    } else {
        println!("cargo:warning=rc.exe failed: the exes keep the default icon");
    }
}

/// The newest Windows 10/11 SDK's x64 rc.exe.
fn find_rc() -> Option<PathBuf> {
    let bin = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut found: Vec<PathBuf> = std::fs::read_dir(bin)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("x64").join("rc.exe"))
        .filter(|p| p.exists())
        .collect();
    found.sort();
    found.pop()
}
