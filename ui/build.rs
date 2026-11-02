// The settings window, with its English bundled: ../i18n/en/LC_MESSAGES/nowbar-ui.po (msgids are
// the Turkish strings themselves, no context; the Rust side's table comes from the same file).
fn main() {
    println!("cargo:rerun-if-changed=../assets/nowbar-256.png");
    println!("cargo:rerun-if-changed=syro-mark.png");
    println!("cargo:rerun-if-changed=settings.slint");
    println!("cargo:rerun-if-changed=pages.slint");
    println!("cargo:rerun-if-changed=pills.slint");
    println!("cargo:rerun-if-changed=store.slint");
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("../i18n")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("main.slint", config).expect("ui/main.slint");
}
