use std::path::PathBuf;

use serde::Deserialize;

use crate::error::Error;

#[derive(Clone, Debug, Deserialize)]
crate struct Config {
    #[serde(default = "default_address")]
    crate address: String,
    #[serde(default = "default_assets_dir")]
    crate assets_dir: PathBuf,
    #[serde(skip)]
    crate static_dir: PathBuf,
    #[serde(default = "default_max_inline_size")]
    crate max_inline_size: u64,
    #[serde(default)]
    crate disable_preload: bool,
    #[serde(default)]
    crate enable_inline_css: bool,
    #[serde(default)]
    crate enable_inline_svg: bool,
}

fn default_address() -> String {
    "0.0.0.0:80".into()
}

fn default_assets_dir() -> PathBuf {
    "assets".into()
}

const fn default_max_inline_size() -> u64 {
    12 * 1024
}

impl Config {
    /// Coerces a configuration source to a website config instance.
    crate fn new<T>(source: T) -> Result<Self, Error>
    where
        T: 'static,
        T: config::Source + Send + Sync,
    {
        let mut config = config::Config::default();

        config.merge(source)?;

        let mut config = config.try_into::<Self>()?;

        config.static_dir = config.assets_dir.clone();
        config.static_dir.push("static");

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use crate::config::Config;

    #[test]
    fn test_new_config() {
        use config::{File, FileFormat::Yaml};

        const ADDRESS: &str = "127.0.0.1:8080";
        const ASSETS_DIR: &str = "/home/me/files";
        const MAX_INLINE_SIZE: u64 = 32 * 1024;
        const DISABLE_PRELOAD: bool = true;
        const ENABLE_INLINE_CSS: bool = true;
        const ENABLE_INLINE_SVG: bool = true;

        let source = File::from_str(
            format!(
                r#"
address: {}
assets_dir: {}
max_inline_size: {}
disable_preload: {}
enable_inline_css: {}
enable_inline_svg: {}
        "#,
                ADDRESS,
                ASSETS_DIR,
                MAX_INLINE_SIZE,
                DISABLE_PRELOAD,
                ENABLE_INLINE_CSS,
                ENABLE_INLINE_SVG,
            )
            .as_str(),
            Yaml,
        );
        let config = Config::new(source).expect("Failed to get config");

        assert_eq!(ADDRESS, config.address);
        assert_eq!(ASSETS_DIR, config.assets_dir.to_str().unwrap());
        assert_eq!(MAX_INLINE_SIZE, config.max_inline_size);
        assert_eq!(DISABLE_PRELOAD, config.disable_preload);
        assert_eq!(ENABLE_INLINE_CSS, config.enable_inline_css);
        assert_eq!(ENABLE_INLINE_SVG, config.enable_inline_svg);
        assert_eq!(
            format!("{}/{}", ASSETS_DIR, "static"),
            config.static_dir.to_str().unwrap()
        );
    }
}
