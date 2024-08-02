use process_frontmatter::FrontmatterPreprocessor;

/// Main function for preprocessing data in frontmatter
fn main() {
    // capture args from env
    let args: Vec<String> = std::env::args().collect();

    // Check if the first argument is "supports" and the second argument exists
    //
    // mdbook make two preprocessing requests:
    // 1) check that the renderer is supported
    // 2) expects json from stdin
    if args.len() > 2 && args[1] == "supports" {
        // Check if the preprocessor supports the specified renderer
        match args[2].as_str() {
            // supports HTML renderer
            "html" => std::process::exit(0),
            // untested
            _ => std::process::exit(1),
        }
    } else {
        // Normal operation, not checking for renderer support
        let backend = FrontmatterPreprocessor::default();
        if let Err(e) = backend.handle_preprocessing() {
            eprintln!("Error processing frontmatter: {:?}", e);
            std::process::exit(1);
        }
    }
}
