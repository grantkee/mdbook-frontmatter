//! Preprocess key/values in-between "+++" as frontmatter.
use mdbook::book::Book;
use mdbook::errors::Error;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext};
use mdbook::BookItem;
use pulldown_cmark::{CowStr, Event, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;
use regex::{Captures, Regex};
use semver::{Version, VersionReq};
use std::io;

#[derive(Default)]
pub struct FrontmatterPreprocessor;

impl FrontmatterPreprocessor {
    /// Preprocess book content.
    ///
    /// This method calls the impl `run` method for [Self] to edit content
    /// and return the processed [Book] to stdout.
    pub fn handle_preprocessing(&self) -> Result<(), Error> {
        let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

        let book_version = Version::parse(&ctx.mdbook_version)?;
        let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

        if !version_req.matches(&book_version) {
            // attempt to log error
            eprintln!(
                "Warning: The {} plugin was built against version {} of mdbook, \
                 but we're being called from version {}",
                self.name(),
                mdbook::MDBOOK_VERSION,
                ctx.mdbook_version
            );
        }

        // process book and return frontmatter
        let processed_book = self.run(&ctx, book)?;
        serde_json::to_writer(io::stdout(), &processed_book)?;
        Ok(())
    }
}

impl Preprocessor for FrontmatterPreprocessor {
    fn name(&self) -> &str {
        "frontmatter-preprocessor"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        // NOTE: "---" is interpreted as Header, so use "+++"
        let frontmatter_delimiter = CowStr::Borrowed("+++");

        // loop through each book item to parse chapters
        book.for_each_mut(|item| {
            // only parse chapters
            if let BookItem::Chapter(chapter) = item {
                // flag for capturing frontmatter
                let mut capture = false;
                let mut frontmatter_collection = vec![];
                let mut formatted_content = vec![];

                // create markdown parser for events
                let parser = pulldown_cmark::Parser::new(&chapter.content);

                // loop through events to find frontmatter section based on delimiter
                for event in parser {
                    match event {
                        // handle delimiter
                        Event::Text(ref text) if text == &frontmatter_delimiter => {
                            // first time seeing delimiter, this is false
                            // second time, construct table with captured frontmatter
                            if capture {
                                let frontmatter = parse_frontmatter(&frontmatter_collection);
                                let html_table = create_html_table_events(frontmatter);

                                // concat doesn't work
                                for event in html_table {
                                    formatted_content.push(event);
                                }
                                frontmatter_collection.clear();
                            }
                            // turn capture flag "true"
                            //
                            // and don't capture the delimiter event
                            capture = !capture;
                        }
                        // capture content within frontmatter delimiters
                        Event::Text(content) if capture => {
                            frontmatter_collection.push(content.to_string())
                        }
                        // avoid capturing "SoftBreak", etc. in frontmatter
                        _ if !capture => formatted_content.push(event),
                        // ignore "SoftBreak"s in frontmatter section
                        _ => (),
                    }
                }

                // replace chapter content with formatted content
                let mut buf = String::with_capacity(chapter.content.len());
                chapter.content = cmark(formatted_content.iter(), &mut buf)
                    .map(|_| buf)
                    .expect("Markdown serialization failed")
            }
        });

        Ok(book)
    }
}

/// Create key/values for frontmatter by splitting ":" and trimming whitespace.
///
/// Use a `Vec` so the order is preserved.
fn parse_frontmatter(frontmatter_text: &[String]) -> Vec<(String, String)> {
    frontmatter_text
        .iter()
        .filter_map(|line| {
            // separate by colon + space
            let parts: Vec<_> = line.splitn(2, ':').collect();

            if parts.len() == 2 {
                Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
            } else {
                None
            }
        })
        .collect()
}

/// Create owned events for table html.
///
/// The events are created for use with pulldown cmark.
///
/// There may be a better way to do this, but this seems sturdy.
fn create_html_table_events<'a>(frontmatter: Vec<(String, String)>) -> Vec<Event<'a>> {
    // create events for cmark
    let mut events = vec![];
    // start tag
    events.push(Event::Start(Tag::HtmlBlock));
    // create table
    events.push(Event::Html(CowStr::Boxed(
        "<table class=\"preamble\">\n".into(),
    )));
    // loop through frontmatter to create table rows
    for (key, mut value) in frontmatter {
        // create links for github/email
        if key == "author" {
            value = linkify_text(&value)
        }

        events.push(Event::Html(CowStr::Boxed(
            format!("<tr><th>{}</td><td>{}</td></tr>\n", key, value).into(),
        )));
    }
    // close table
    events.push(Event::Html(CowStr::Boxed("</table>\n".into())));
    // end tag
    events.push(Event::End(TagEnd::HtmlBlock));
    events
}

/// Create anchor tags for github usernames and emails inside frontmatter.
fn linkify_text(text: &str) -> String {
    // Regex to find GitHub usernames and emails
    let github_regex = Regex::new(r"\(@([a-zA-Z0-9_]+)\)").expect("github regex");
    let email_regex =
        Regex::new(r"\(([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+)\)").expect("email regex");

    // Replace GitHub usernames with links
    let text = github_regex.replace_all(text, |caps: &Captures| {
        format!("(<a href=\"https://github.com/{0}\">@{0}</a>)", &caps[1])
    });

    // Replace emails with mailto links
    let text = email_regex.replace_all(&text, |caps: &Captures| {
        format!("(<a href=\"mailto:{}\">{}</a>)", &caps[1], &caps[1])
    });

    text.to_string()
}
