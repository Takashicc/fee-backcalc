fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.png");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    if let Err(err) = configure_windows_icon() {
        panic!("failed to configure Windows icon: {err}");
    }
}

fn configure_windows_icon() -> Result<(), Box<dyn std::error::Error>> {
    use image::{ExtendedColorType, ImageEncoder, codecs::ico::IcoEncoder, imageops::FilterType};
    use std::{env, fs::File, path::Path};

    let png_path = Path::new("assets").join("app-icon.png");

    let image = image::open(&png_path)?
        .resize(256, 256, FilterType::Lanczos3)
        .into_rgba8();
    let (width, height) = image.dimensions();
    let icon_path = Path::new(&env::var("OUT_DIR")?).join("app-icon.ico");
    let file = File::create(&icon_path)?;

    IcoEncoder::new(file).write_image(image.as_raw(), width, height, ExtendedColorType::Rgba8)?;

    winresource::WindowsResource::new()
        .set_icon(icon_path.to_string_lossy().as_ref())
        .compile()?;

    Ok(())
}
