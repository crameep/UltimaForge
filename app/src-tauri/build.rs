fn main() {
    // Override Tauri's default Windows application manifest with one that requests
    // administrator elevation. UAC prompts once at startup (before the window opens)
    // so the NSIS self-update installer inherits the elevated token and doesn't
    // show a second background UAC dialog during an update.
    #[cfg(target_os = "windows")]
    {
        let attrs = tauri_build::Attributes::new().windows_attributes(
            tauri_build::WindowsAttributes::new()
                .app_manifest(include_str!("app.manifest")),
        );
        tauri_build::try_build(attrs).expect("failed to run tauri-build");
        return;
    }

    tauri_build::build()
}
