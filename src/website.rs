use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::str;

use actix_web::http::header::{
    HeaderValue, IntoHeaderValue, CONTENT_SECURITY_POLICY, LINK, REFERRER_POLICY,
    STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS, X_XSS_PROTECTION,
};
use actix_web::web::{get, HttpResponse};
use actix_web::Route;
use html5minify::minify;
use lazy_static::lazy_static;
use percent_encoding::percent_decode;
use pulldown_cmark::{html::write_html, Event, Parser, Tag};
use regex::Regex;

use crate::config::Config;
use crate::error::Error;

lazy_static! {
    static ref RE_IMPORT: Regex = Regex::new(r"@import\s+url\([^\)]+\);?").unwrap();
    static ref RE_SVG: Regex = Regex::new(r#"\s+(version|xmlns)="[^"]+""#).unwrap();
}

/// A markdown to HTML website implementation.
#[derive(Clone)]
pub struct Website {
    config: Config,
}

type ParseResult<'a> = Result<(Vec<Event<'a>>, Vec<HeaderValue>), Error>;

impl Website {
    crate const fn new(config: Config) -> Self {
        Self { config }
    }

    crate fn routes(&self) -> Result<Vec<(String, Route)>, Error> {
        let mut static_files: HashSet<String> = HashSet::new();

        for entry in fs::read_dir(&self.config.static_dir)? {
            let path = entry?.path();

            if path.is_file() {
                static_files.insert(path.file_name()?.to_str()?.to_owned());
            }
        }

        let path = path_append(&self.config.assets_dir, "md");
        let prefix = path.to_str()?;
        let md_ext = Some(OsStr::new("md"));
        let mut routes = vec![];

        for entry in fs::read_dir(&path)? {
            let path = entry?.path();

            if path.is_file() && path.extension() == md_ext {
                routes.push(self.route(&path, &static_files, prefix)?);
            }
        }

        Ok(routes)
    }

    fn route(
        &self,
        path: &PathBuf,
        static_files: &HashSet<String>,
        prefix: &str,
    ) -> Result<(String, Route), Error> {
        let markdown = read_to_string(path)?;
        let (events, mut preload_headers) = self.parse_markdown(&markdown)?;
        let path = path
            .to_str()?
            .trim_end_matches(".md")
            .trim_start_matches(prefix)
            .trim_start_matches('/')
            .to_owned();
        let css_file_name = {
            let file_name = path.clone() + ".css";

            if static_files.contains(file_name.as_str()) {
                file_name
            } else {
                "main.css".into()
            }
        };
        let mut output = b"<!doctype html>\n".to_vec();
        let css_path = &path_append(&self.config.static_dir, &css_file_name);
        let mut write_stylesheet = |url: &str| -> Result<(), Error> {
            writeln!(&mut output, "<link rel=stylesheet href=\"{}\">", url)?;

            if !self.config.disable_preload && url.starts_with('/') {
                preload_headers.insert(0, http_preload(url, "style")?);
            }

            Ok(())
        };

        // Use inline style for small CSS files
        if !self.config.enable_inline_css || file_size(css_path)? > self.config.max_inline_size {
            write_stylesheet(&css_file_name)?;
        } else {
            let css = read_to_string(css_path)?;
            let (css, urls) = import_urls(&css)?;

            for url in &urls {
                write_stylesheet(url)?;
            }

            writeln!(&mut output, "<style>{}</style>", css.trim_end())?;
        }

        write_html(&mut output, events.into_iter())?;

        let js_file_name = &(path.clone() + ".js");

        if static_files.contains(js_file_name.as_str()) {
            write_js(&mut output, js_file_name)?;

            if !self.config.disable_preload {
                preload_headers.insert(0, http_preload(js_file_name, "script")?);
            }
        }

        if static_files.contains("favicon.ico") {
            preload_headers.push(http_preload("favicon.ico", "image")?);
        }

        let mut minified = Vec::new();

        minify(&mut output.as_slice(), &mut minified)?;

        Ok((
            if path == "index" { "/".into() } else { path },
            get().to(move || {
                let mut response = HttpResponse::Ok();

                for value in preload_headers.clone() {
                    response.header(LINK, value);
                }

                response
                    .header(CONTENT_SECURITY_POLICY, content_security_policy())
                    .header(
                        REFERRER_POLICY,
                        "no-referrer, strict-origin-when-cross-origin",
                    )
                    .header(STRICT_TRANSPORT_SECURITY, "max-age=63072000")
                    .header(X_CONTENT_TYPE_OPTIONS, "nosniff")
                    .header(X_FRAME_OPTIONS, "SAMEORIGIN")
                    .header(X_XSS_PROTECTION, "1; mode=block")
                    .content_type("text/html; charset=utf-8")
                    .body(minified.clone())
            }),
        ))
    }

    /// Parses the markdown string and process the events to determine
    /// pre-loadable images and the page's title.
    fn parse_markdown<'a>(&self, markdown: &'a str) -> ParseResult<'a> {
        let mut events = Vec::new();
        let mut preload_headers = Vec::new();
        let mut svg: Option<String> = None;

        for event in Parser::new(markdown) {
            match (&event, events.last(), &svg) {
                (Event::Start(Tag::Image(_, ref src, ref title)), ..) if src.starts_with('/') => {
                    if self.config.enable_inline_svg && src.ends_with(".svg") {
                        let path = &path_append(
                            &self.config.static_dir,
                            &percent_decode(src[1..].as_bytes()).decode_utf8()?,
                        );

                        if file_size(path)? <= self.config.max_inline_size {
                            svg = Some(inline_svg(path, title)?);

                            continue;
                        }
                    }

                    if !self.config.disable_preload {
                        preload_headers.push(http_preload(&src[1..], "image")?);
                    }
                }
                (_, _, Some(content)) => {
                    events.push(Event::Html(content.to_owned().into()));

                    svg = None;

                    continue;
                }
                _ => {}
            }

            events.push(event);
        }

        Ok((events, preload_headers))
    }
}

/// Returns the content security policy for DEBUG and RELEASE builds.
/// RELEASE assumes assets are limited to HTTPS.
const fn content_security_policy() -> &'static str {
    if cfg!(debug_assertions) {
        "default-src * 'unsafe-inline'; object-src 'none'; frame-ancestors 'none'; base-uri 'none'"
    } else {
        "default-src https: 'unsafe-inline'; object-src 'none'; frame-ancestors 'none'; base-uri 'none'"
    }
}

fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, Error> {
    let mut f = File::open(path)?;
    let mut content = String::new();

    f.read_to_string(&mut content)?;

    Ok(content.trim_end().into())
}

fn file_size<P: AsRef<Path>>(path: P) -> Result<u64, io::Error> {
    Ok(fs::metadata(path)?.len())
}

fn path_append<T: Into<PathBuf>>(root: T, path: &str) -> PathBuf {
    let mut p = root.into();

    p.push(path);
    p
}

fn http_preload(file_name: &str, as_type: &str) -> Result<HeaderValue, Error> {
    Ok(format!("</{}>; rel=preload; as={}", file_name, as_type).try_into()?)
}

/// Extracts @import CSS files and removes them from the original CSS source.
/// Reduces HTTP request indirection for imported CSS files.
fn import_urls<'a>(css: &'a str) -> Result<(Cow<'a, str>, Vec<String>), Error> {
    use cssparser::{Parser, ParserInput, Token};

    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    let mut previous: Option<Token> = None;
    let mut urls = Vec::new();

    while let Ok(token) = parser.next() {
        match (token, &previous) {
            (Token::Function(ref name), Some(Token::AtKeyword(ref keyword)))
                if keyword.eq_ignore_ascii_case("import") && name.eq_ignore_ascii_case("url") =>
            {
                if let Err(e) = parser.parse_nested_block::<_, _, Error>(|parser| {
                    urls.push(parser.expect_string()?.to_string());

                    Ok(())
                }) {
                    return Err(Error::CssParse(format!("{:?}", e)));
                }
            }
            _ => previous = Some(token.clone()),
        }
    }

    // TODO: build output via CSS AST instead of doing a regex replace
    Ok((RE_IMPORT.replace_all(css, ""), urls))
}

fn inline_svg<P: AsRef<Path>>(path: P, title: &str) -> Result<String, Error> {
    const NEEDLE: &str = "<svg";

    let svg = read_to_string(path)?;
    let svg = RE_SVG.replace_all(&svg, "");

    if title == "" || svg.contains("<title>") {
        return Ok(svg.into());
    }

    let from = svg.find(NEEDLE)?;
    let tail = &svg[from + NEEDLE.len()..];
    let i = tail.find('>')?;
    let mut svg = svg.to_string();

    svg.insert_str(
        from + NEEDLE.len() + i + 1,
        &format!("<title>{}</title>", title),
    );

    Ok(svg)
}

/// Writes JavaScript to asynchronously load a page's JavaScript file.
fn write_js(buf: &mut dyn Write, file_name: &str) -> io::Result<()> {
    writeln!(
        buf,
        "<script>\
         (function(d,src){{\
         var e=d.createElement('script'),s=d.getElementsByTagName('script')[0];\
         e.src=src;\
         e.async=1;\
         s.parentNode.insertBefore(e,s);\
         }})(document,'{}');\
         </script>",
        file_name
    )
}

#[cfg(test)]
mod tests {
    extern crate test;

    use config::{File, FileFormat::Yaml};

    use super::*;
    use crate::config::Config;

    #[test]
    fn test_parse_markdown() {
        let source = File::from_str("disable_inline_svg: true", Yaml);
        let config = Config::new(source).expect("Failed to get config");
        let markdown = read_to_string("assets/md/index.md").expect("Failed to read markdown file");
        let (events, preload_headers) = Website::new(config)
            .parse_markdown(&markdown)
            .expect("Failed to parse markdown");

        assert_ne!(0, events.len());
        assert_ne!(0, preload_headers.len());
    }
}
