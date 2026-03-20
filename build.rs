fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app-icon.ico");
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
