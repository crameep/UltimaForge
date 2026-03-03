fn main() {
    // Embed Windows application manifest (requestedExecutionLevel = requireAdministrator).
    // This causes the UAC prompt to appear once at startup before the window opens,
    // so the NSIS self-update installer inherits the elevated token and doesn't
    // produce a second background UAC dialog during an update.
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_manifest(include_str!("app.manifest"));
        res.compile().unwrap();
    }

    tauri_build::build()
}
