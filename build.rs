use embed_resource;

fn main() {
    println!("cargo:rerun-if-changed=interceptor.asm");
    println!("cargo:rustc-env=CARGO_CFG_TARGET_FEATURE=fxsr,sse,sse2,avx");
    cc::Build::new()
        .file("src/injection.asm")
        .compile("injection");
    embed_resource::compile("res.rc");
}
