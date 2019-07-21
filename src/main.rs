#![feature(crate_visibility_modifier, test, try_trait)]

mod config;
mod error;
mod website;

use actix_files::Files;
use actix_web::{App, HttpServer};

use crate::config::Config;
use crate::error::Error;
use crate::website::Website;

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
