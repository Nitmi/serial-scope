fn main() {
    #[cfg(windows)]
    {
        let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_owned());
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app-icon.ico");
        res.set("ProductName", "Serial Scope");
        res.set("FileDescription", "Serial Scope");
        res.set("OriginalFilename", "serial-scope.exe");
        res.set("CompanyName", "Nitmi");
        res.set("ProductVersion", &version);
        res.set("FileVersion", &version);
        if let Err(err) = res.compile() {
            panic!("failed to compile Windows resources: {err}");
        }
    }

    #[cfg(not(windows))]
    {
        println!("cargo:rerun-if-changed=assets/app-icon.png");
    }

    println!("cargo:rerun-if-changed=assets/app-icon.ico");
    println!("cargo:rerun-if-changed=assets/app-icon.png");
}
