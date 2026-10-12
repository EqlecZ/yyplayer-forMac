use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env != "msvc" {
        println!(
            "cargo:warning=The YYPlayer PE icon resource is currently configured for Windows MSVC builds."
        );
        return;
    }

    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"));
    let icon_path = manifest_dir.join("../../assets/icons/yyplayer/liquid-orbit-disc/yyplayer.ico");
    println!("cargo:rerun-if-changed={}", icon_path.display());
    if !icon_path.is_file() {
        panic!(
            "YYPlayer Windows icon is missing: {}. Run scripts/Build-AppIcon.ps1 first.",
            icon_path.display()
        );
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    let resource_script = out_dir.join("yyplayer-icon.rc");
    let resource_file = out_dir.join("yyplayer-icon.res");
    let escaped_icon_path = icon_path.to_string_lossy().replace('\\', "\\\\");
    let version = env::var("CARGO_PKG_VERSION").expect("Cargo sets CARGO_PKG_VERSION");
    let version_numbers = ["MAJOR", "MINOR", "PATCH"].map(|part| {
        env::var(format!("CARGO_PKG_VERSION_{part}"))
            .expect("Cargo sets the package version components")
            .parse::<u16>()
            .expect("Windows version components must fit in 16 bits")
    });
    let [major, minor, patch] = version_numbers;
    fs::write(
        &resource_script,
        format!(
            // RC defaults to the system code page; Rust writes this file as UTF-8.
            // Declare it explicitly so an icon path containing Chinese remains valid.
            r#"#pragma code_page(65001)
IDI_ICON1 ICON "{escaped_icon_path}"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "FileDescription", "YYPlayer\0"
            VALUE "FileVersion", "{version}\0"
            VALUE "InternalName", "yyplayer\0"
            VALUE "LegalCopyright", "GPL-3.0-only\0"
            VALUE "OriginalFilename", "yyplayer.exe\0"
            VALUE "ProductName", "YYPlayer\0"
            VALUE "ProductVersion", "{version}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
        ),
    )
    .expect("write generated Windows icon and version resource script");

    let compiler = find_resource_compiler().unwrap_or_else(|| {
        panic!("Windows MSVC icon embedding requires rc.exe or llvm-rc.exe from the installed Windows SDK.")
    });
    let status = Command::new(&compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource_file)
        .arg(&resource_script)
        .status()
        .unwrap_or_else(|error| panic!("run {}: {error}", compiler.display()));
    if !status.success() {
        panic!(
            "{} failed to compile the YYPlayer icon resource: {status}",
            compiler.display()
        );
    }

    println!(
        "cargo:rustc-link-arg-bin=yyplayer={}",
        resource_file.display()
    );
}

fn find_resource_compiler() -> Option<PathBuf> {
    if let Some(configured) = env::var_os("RC") {
        let configured = PathBuf::from(configured);
        if configured.is_file() {
            return Some(configured);
        }
        if let Some(found) = find_on_path(configured.to_string_lossy().as_ref()) {
            return Some(found);
        }
    }

    if let Some(found) = find_on_path("rc.exe") {
        return Some(found);
    }

    for sdk_root in windows_sdk_roots() {
        if let Some(found) = find_in_sdk(&sdk_root) {
            return Some(found);
        }
    }

    find_on_path("llvm-rc.exe")
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let output = Command::new("where.exe").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

fn windows_sdk_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = env::var_os("WindowsSdkDir") {
        roots.push(PathBuf::from(root));
    }
    if let Ok(output) = Command::new("reg.exe")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Windows Kits\Installed Roots",
            "/v",
            "KitsRoot10",
        ])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        if let Some(root) = text
            .lines()
            .find(|line| line.contains("KitsRoot10"))
            .and_then(|line| line.split_once("REG_SZ"))
            .map(|(_, value)| value.trim())
        {
            roots.push(PathBuf::from(root));
        }
    }
    roots
}

fn find_in_sdk(root: &Path) -> Option<PathBuf> {
    let bin = root.join("bin");
    for architecture in ["x64", "x86"] {
        let direct = bin.join(architecture).join("rc.exe");
        if direct.is_file() {
            return Some(direct);
        }
    }

    let mut versions = fs::read_dir(&bin)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    versions.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    for version in versions {
        for architecture in ["x64", "x86"] {
            let candidate = version.join(architecture).join("rc.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
