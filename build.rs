fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "JumpChamp");
        res.set("FileDescription", "JumpChamp Prime Gap Explorer");
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resource: {}", e);
            std::process::exit(1);
        }
    }
}
