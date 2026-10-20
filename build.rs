fn main() {
    println!("cargo:rerun-if-changed=cpp/optimizer.cpp");
    println!("cargo:rerun-if-changed=cpp/optimizer.h");
    println!("cargo:rerun-if-changed=assets");

    cc::Build::new()
        .cpp(true)
        .file("cpp/optimizer.cpp")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("/EHsc")
        .flag_if_supported("/utf-8")
        .compile("optimizer");

    for lib in ["psapi", "shell32", "powrprof", "advapi32", "user32", "pdh"] {
        println!("cargo:rustc-link-lib={lib}");
    }

    // Icône partout ; demande admin seulement en release, pour que debug et tests se lancent sans UAC.
    let rc = if std::env::var("PROFILE").as_deref() == Ok("release") { "assets/app-admin.rc" } else { "assets/app.rc" };
    embed_resource::compile_for(rc, ["Eclair"], embed_resource::NONE).manifest_optional().unwrap();
}
