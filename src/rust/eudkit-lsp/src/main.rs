fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    eudkit_lsp::server::run()
}
