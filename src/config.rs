//! Provides structs to handle Pacstall's configuration.
//!
//! The configuration is first read from the config file located at
//! `/etc/pacstall/config.toml`, then environment variables prefixed with
//! `PACSTALL_` may override the configuration.
//!
//! # Format
//!
//! ```toml
//! [settings]
//! jobs = 10
//! editor = "emacs"
//!
//! [[repositories]]
//! name = "official"
//! url = "https://github.com/pacstall/pacstall-programs"
//! preference = 1
//!
//! [[repositories]]
//! name = "third_party"
//! url = "https://github.com/user/third-party"
//! preference = 2
//! ```
//!
//! # Note
//!
//! The default configuration is used if the config file is not found or is
//! empty.

use std::env;
use std::process::Command;

use figment::providers::{Env, Format, Toml};
use figment::value::{Dict, Map};
use figment::{Error, Figment, Metadata, Profile, Provider};
use serde::{Deserialize, Serialize};

use crate::model::{default_repository, Repository};

/// Pacstall's configuration.
///
/// Gives access to the [configuration](Config) extracted, and the [Figment]
/// used to generate it.
#[derive(Debug)]
pub struct App {
    pub config: Config,
    /// Allows other libraries making use of the framework to also extract
    /// values from the same [Figment].
    pub figment: Figment,
}

impl App {
    /// Generate a new [App] using the default [Provider].
    ///
    /// # Examples
    ///
    /// ```
    /// use libpacstall::config::App;
    ///
    /// let app = App::new().unwrap();
    ///
    /// /// Get the extracted config.
    /// let config = app.config;
    ///
    /// /// Get the figment used to extract the config.
    /// let figment = app.figment;
    /// ```
    ///
    /// # Errors
    ///
    /// Any [Error] occurring while extracting the configuration will be
    /// returned.
    pub fn new() -> Result<App, Error> { App::custom(Config::figment()) }

    /// Generate a new [App] using a custom [Provider].
    ///
    /// # Examples
    ///
    /// ```
    /// use figment::providers::Env;
    /// use libpacstall::config::App;
    ///
    /// /// Use a custom figment provider in your application.
    /// let app = App::custom(Env::prefixed("some_prefix")).unwrap();
    ///
    /// let config = app.config;
    /// ```
    ///
    /// # Errors
    ///
    /// Any [Error] occurring while extracting the configuration will be
    /// returned.
    pub fn custom<T: Provider>(provider: T) -> Result<App, Error> {
        let figment = Figment::from(provider);
        Ok(App {
            config: Config::from(&figment)?,
            figment,
        })
    }
}

/// The extracted configuration.
#[derive(Deserialize, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default = "default_repository")]
    pub repositories: Vec<Repository>,
}

impl Config {
    /// Allow the configuration to be extracted from any [Provider].
    ///
    /// # Examples
    ///
    /// ```
    /// use figment::providers::Env;
    /// use libpacstall::config::Config;
    ///
    /// let config = Config::from(Env::prefixed("some_prefix")).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Any [Error] occurring while extracting the configuration will be
    /// returned.
    pub fn from<T: Provider>(provider: T) -> Result<Config, Error> {
        Figment::from(provider).extract()
    }

    /// Provide a default provider, a `Figment`.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpacstall::config::Config;
    ///
    /// let figment = Config::figment();
    /// ```
    pub fn figment() -> Figment {
        Figment::from(Toml::file("/etc/pacstall/config.toml"))
            .merge(Env::prefixed("PACSTALL_").split("_"))
    }
}

impl figment::Provider for Config {
    fn metadata(&self) -> Metadata { figment::Metadata::named("Pacstall Config") }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        figment::providers::Serialized::defaults(Config::default()).data()
    }

    fn profile(&self) -> Option<Profile> { None }
}

/// The extracted `settings` table.
#[derive(Deserialize, Debug, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// The preferred `editor` set by the user. (Defaults to the environment
    /// variables *EDITOR* -> *VISUAL* -> the editor chosen by the
    /// `sensible-editor` command -> "nano".)
    pub editor: String,
    /// Number of threads to use for building packages (Defaults to the number
    /// of *logical cores* on the computer.)
    pub jobs: u8,
}

impl Default for Settings {
    #[allow(clippy::cast_possible_truncation)]
    fn default() -> Self {
        Self {
            editor: None
                .or_else(|| env::var("EDITOR").ok())
                .or_else(|| env::var("VISUAL").ok())
                .or_else(|| {
                    let home_dir = env::var("HOME").ok()?;
                    let output = Command::new("bash")
                        .current_dir(&home_dir)
                        .args(["-c", "source .selected_editor && echo ${SELECTED_EDITOR}"])
                        .output()
                        .ok()?;

                    if !output.status.success() {
                        return None;
                    }
                    Some(String::from_utf8(output.stdout).ok()?.trim().into())
                })
                .unwrap_or_else(|| "nano".into()),
            jobs: num_cpus::get() as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    use figment::Jail;
    use rstest::rstest;

    use super::*;

    fn run_in_jail<T, U>(config: Option<&str>, jail_modifications: T, code: U)
    where
        T: FnOnce(&mut Jail),
        U: FnOnce(Config) -> Result<(), figment::Error>,
    {
        Jail::expect_with(|jail| {
            jail.create_file(
                "test_config.toml",
                config.unwrap_or({
                    r#"
                    ## This is the boilerplate toml file's contents.
                    # Configure Pacstall settings here.
                    [settings]
                    jobs = 10 # Number of jobs to run in parallel.
                    editor = "nvim" # Editor to use for editing.

                    [[repositories]]
                    name = "official"
                    url = "https://github.com/pacstall/pacstall-programs"
                    preference = 1

                    [[repositories]]
                    name = "unofficial"
                    url = "https://github.com/evil-pacstall/evil-pacstall-programs"
                    preference = 2
                    "#
                }),
            )?;

            jail_modifications(jail);
            code(
                App::custom(Toml::file("test_config.toml"))?
                    .figment
                    .merge(Env::prefixed("TEST_PACSTALL_").split("_"))
                    .extract::<Config>()?,
            )?;

            Ok(())
        });
    }

    #[rstest]
    fn explicit_full_config() {
        run_in_jail(
            None,
            |_| {},
            |config| {
                assert_eq!(
                    config,
                    Config {
                        settings: Settings {
                            editor: "nvim".into(),
                            jobs: 10u8,
                        },
                        repositories: vec![
                            Repository {
                                name: "official".into(),
                                url: "https://github.com/pacstall/pacstall-programs".into(),
                                preference: 1
                            },
                            Repository {
                                name: "unofficial".into(),
                                url: "https://github.com/evil-pacstall/evil-pacstall-programs"
                                    .into(),
                                preference: 2,
                            }
                        ]
                    }
                );

                Ok(())
            },
        );
    }

    /// Monkey patches removing the environment variables specified, and
    /// renaming the `$HOME/.selected_editor` file.
    fn monkeypatch<T>(vars_to_remove: &[&str], test_code: T)
    where
        T: FnOnce() -> Result<(), Box<dyn std::error::Error>>,
    {
        let home = env::var("HOME").unwrap();
        let home = Path::new(&home);

        let collected_vars: Vec<_> = vars_to_remove.iter().map(env::var).collect();
        vars_to_remove.iter().for_each(env::remove_var);

        let selected_editor_path = home.join(".selected_editor");
        let tmp_selected_editor_path = home.join(".test_tmp_selected_editor");

        if selected_editor_path.exists() {
            fs::rename(&selected_editor_path, &tmp_selected_editor_path).unwrap();
        }

        test_code().unwrap();

        if tmp_selected_editor_path.exists() {
            fs::rename(&tmp_selected_editor_path, &selected_editor_path).unwrap();
        }

        collected_vars
            .iter()
            .zip(vars_to_remove)
            .for_each(|(collected_var, var_to_set_back)| {
                if collected_var.is_ok() {
                    env::set_var(collected_var.as_ref().unwrap(), var_to_set_back);
                }
            });
    }

    #[rstest]
    #[allow(clippy::cast_possible_truncation)]
    fn default_full_config() {
        Jail::expect_with(|jail| {
            let figment = App::custom(Toml::file("config.toml"))?
                .figment
                .merge(Env::prefixed("TEST_PACSTALL_").split("_"));

            let config: Config = figment.extract()?;

            assert_eq!(config.settings.jobs, num_cpus::get() as u8);

            assert_eq!(
                config.repositories,
                vec![Repository {
                    name: "official".into(),
                    url: "https://github.com/pacstall/pacstall-programs".into(),
                    preference: 1
                }]
            );

            // Test that reading the `EDITOR` environment variable works

            monkeypatch(&["VISUAL"], || {
                jail.set_env("EDITOR", "emacs");
                let config: Config = figment.extract()?;

                assert_eq!(config.settings.editor, "emacs");

                Ok(())
            });

            // Test that reading the `VISUAL` environment variable works

            monkeypatch(&["EDITOR"], || {
                jail.set_env("VISUAL", "emacs");
                let config: Config = figment.extract()?;

                assert_eq!(config.settings.editor, "emacs");

                Ok(())
            });

            // Test that parsing the `.selected_editor` file works

            monkeypatch(&["EDITOR", "VISUAL"], || {
                let home = env::var("HOME")?;
                let home = Path::new(&home);

                let selected_editor_path = &home.join(".selected_editor");

                writeln!(
                    File::create(&selected_editor_path)?,
                    r#"
                    # This is a mock file, if this persists on your system contact the Pacstall developers.
                    SELECTED_EDITOR="/usr/bin/nvim"
                    "#
                )?;

                let config: Config = figment.extract()?;

                assert_eq!(config.settings.editor, "/usr/bin/nvim");

                fs::remove_file(selected_editor_path)?;

                Ok(())
            });

            // Test that it ultimately defaults to `nano`

            monkeypatch(&["EDITOR", "VISUAL"], || {
                let config: Config = figment.extract()?;

                assert_eq!(config.settings.editor, "nano");

                Ok(())
            });

            Ok(())
        });
    }

    #[rstest]
    fn overriding_config_via_env_vars() {
        run_in_jail(
            None,
            |jail| {
                jail.set_env("TEST_PACSTALL_SETTINGS_EDITOR", "emacs");
                jail.set_env("TEST_PACSTALL_SETTINGS_JOBS", 20);

                jail.set_env(
                    "TEST_PACSTALL_REPOSITORIES",
                    r#"[{name = "foo", url = "bar", preference = 3}]"#,
                );
            },
            |config| {
                assert_eq!(
                    config,
                    Config {
                        settings: Settings {
                            editor: "emacs".into(),
                            jobs: 20u8
                        },
                        repositories: vec![Repository {
                            name: "foo".into(),
                            url: "bar".into(),
                            preference: 3,
                        }],
                    }
                );
                Ok(())
            },
        );
    }

    #[rstest]
    fn provider_implementation() {
        run_in_jail(
            None,
            |_| {},
            |config| {
                let metadata = config.metadata();
                assert_eq!(metadata.name, "Pacstall Config");
                assert_eq!(metadata.provide_location, None);
                assert_eq!(metadata.source, None);

                config.data().unwrap();

                assert_eq!(config.profile(), None);
                Ok(())
            },
        );
    }

    #[rstest]
    #[should_panic]
    fn unknown_fields() {
        run_in_jail(
            Some(
                r#"
                [settings]
                best_programming_language = "rust"
                "#,
            ),
            |_| {},
            |_| Ok(()),
        );
    }

    #[rstest]
    #[should_panic]
    fn missing_fields() {
        run_in_jail(
            Some(
                r#"
                [[repositories]]
                "#,
            ),
            |_| {},
            |_| Ok(()),
        );
    }
}
