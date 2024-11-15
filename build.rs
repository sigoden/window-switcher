fn main() {
    embed_resource::compile("assets/app.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("Failed to compile resource file");
}
