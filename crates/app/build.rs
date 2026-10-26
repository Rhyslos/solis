//! Builds the logo into the program file on Windows, so Explorer, the
//! desktop and the Start menu show it (branding/logo.ico, made from
//! branding/logo.svg).

fn main() {
    println!("cargo:rerun-if-changed=../../branding/logo.ico");
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../branding/logo.ico");
        if let Err(why) = resource.compile() {
            // A program without its icon still runs; say why and carry on.
            println!("cargo:warning=the program's icon wasn't built in: {why}");
        }
    }
}
