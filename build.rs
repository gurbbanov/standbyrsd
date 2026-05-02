fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/standbyrsd-icon.ico");
        res.compile().unwrap();
    }
}
