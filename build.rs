use std::env;
use std::fs::File;
use std::path::Path;

const ICON_SIZES: &[u32] = &[16, 32, 48, 128, 256];

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=docs/app-icon.png");

    let png_bytes = include_bytes!("docs/app-icon.png");
    let source = image::load_from_memory(png_bytes)
        .expect("failed to decode docs/app-icon.png")
        .into_rgba8();

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for &size in ICON_SIZES {
        let resized = image::imageops::resize(
            &source,
            size,
            size,
            image::imageops::FilterType::Lanczos3,
        );
        let icon_image = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
        icon_dir
            .add_entry(ico::IconDirEntry::encode(&icon_image).expect("failed to encode icon frame"));
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let ico_path = Path::new(&out_dir).join("app-icon.ico");
    let ico_file = File::create(&ico_path).expect("failed to create app-icon.ico");
    icon_dir
        .write(ico_file)
        .expect("failed to write app-icon.ico");

    winresource::WindowsResource::new()
        .set_icon(ico_path.to_str().expect("OUT_DIR is not valid UTF-8"))
        .compile()
        .expect("failed to embed Windows icon resource");
}
