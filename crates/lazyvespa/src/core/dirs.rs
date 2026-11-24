//! Directories of lazyvespa, following the
//! [XDG base directories](https://specifications.freedesktop.org/basedir-spec/latest/) on
//! every platform. Their functions return `None` when the home directory cannot be found.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Directory of the configuration, `$XDG_CONFIG_HOME/lazyvespa` or `~/.config/lazyvespa`.
pub fn config_dir() -> Option<PathBuf> {
    app_dir(env::var_os("XDG_CONFIG_HOME"), env::home_dir(), ".config")
}

/// Directory of the data kept between runs, like the query history,
/// `$XDG_DATA_HOME/lazyvespa` or `~/.local/share/lazyvespa`.
pub fn data_dir() -> Option<PathBuf> {
    app_dir(
        env::var_os("XDG_DATA_HOME"),
        env::home_dir(),
        ".local/share",
    )
}

/// Directory of lazyvespa inside the base directory set in the XDG variable, or in its
/// default location inside the home.
fn app_dir(xdg_dir: Option<OsString>, home: Option<PathBuf>, default: &str) -> Option<PathBuf> {
    let base = match xdg_dir {
        // The specification asks to ignore relative paths
        Some(dir) if Path::new(&dir).is_absolute() => PathBuf::from(dir),
        _ => home?.join(default),
    };
    Some(base.join("lazyvespa"))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_dir_in_the_home() {
        let dir = app_dir(None, Some("/home/me".into()), ".config");
        assert_eq!(dir, Some("/home/me/.config/lazyvespa".into()));
    }

    #[test]
    fn test_dir_in_the_xdg_dir() {
        let dir = app_dir(Some("/xdg".into()), Some("/home/me".into()), ".config");
        assert_eq!(dir, Some("/xdg/lazyvespa".into()));
    }

    #[test]
    fn test_relative_xdg_dir_is_ignored() {
        let dir = app_dir(
            Some("relative".into()),
            Some("/home/me".into()),
            ".local/share",
        );
        assert_eq!(dir, Some("/home/me/.local/share/lazyvespa".into()));
        assert_eq!(app_dir(Some("relative".into()), None, ".config"), None);
    }
}
