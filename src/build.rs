fn main() {
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();

        resource.set_icon("assets/favicon.ico");
        resource.set("ProductName", "Vodia PBX Installer Wizard");
        resource.set("FileDescription", "Vodia PBX Installer Wizard");
        resource.set("CompanyName", "Vodia Networks");
        resource.set("LegalCopyright", "Copyright (c) Vodia Networks 2026");
        resource.set("OriginalFilename", "Vodia-PBX-Installer.exe");

        resource
            .compile()
            .expect("Failed to compile Windows resources");
    }
}