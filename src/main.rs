#![feature(crate_visibility_modifier, test, try_trait)]
#![warn(missing_docs)]
#![deny(warnings, clippy::pedantic, clippy::nursery)]

//! Simple MD to HTML website.

mod config;
mod error;
mod website;

use actix_files::Files;
use actix_web::{App, HttpServer};

use crate::config::Config;
use crate::error::Error;
use crate::website::Website;

#[allow(clippy::redundant_clone)]
fn main() -> Result<(), Error> {
    let config = Config::new(::config::Environment::new())?;
    let address = &config.address.clone();
    let website = Website::new(config.clone());

    Ok(HttpServer::new(move || {
        website
            .routes()
            .expect("Failed to get routes")
            .into_iter()
            .fold(App::new(), |app, (path, route)| app.route(&path, route))
            .service(
                Files::new("/", config.static_dir.clone())
                    .use_etag(true)
                    .use_last_modified(true),
            )
    })
    .bind(address)?
    .run()?)
}
