#![allow(clippy::match_on_vec_items)]
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use miette::{Context, IntoDiagnostic, Report, SourceSpan};
use regex::Regex;
use semver::VersionReq;
use spdx::Expression;
use strum::{Display, EnumString};
use tree_sitter::{Node, Parser, Query, QueryCursor};

use super::errors::{BadSyntax, FieldError, MissingField, ParseError};

#[derive(Debug, PartialEq, Eq)]
pub struct Pkgname(String);

impl Pkgname {
    pub(crate) fn new(
        name: &str,
        field_node: &Node,
        value_node: &Node,
    ) -> Result<Self, FieldError> {
        if name.trim().is_empty() {
            return Err(FieldError {
                field_label: "Cannot be empty".into(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (
                    value_node.start_byte(),
                    value_node.end_byte() - value_node.start_byte(),
                )
                    .into(),
                help: "Remove this empty field".into(),
            });
        }
        for (index, character) in name.chars().enumerate() {
            if index == 0 {
                if character == '-' {
                    return Err(FieldError {
                        field_label: "Cannot start with a hyphen ( - )".into(),
                        field_span: (
                            field_node.start_byte(),
                            field_node.end_byte() - field_node.start_byte(),
                        )
                            .into(),
                        error_span: (value_node.start_byte() + 1).into(),
                        help: format!(
                            "Use \x1b[0;32mpkgname=\"{}\"\x1b[0m instead",
                            &name[1..name.len()]
                        ),
                    });
                }

                if character == '.' {
                    return Err(FieldError {
                        field_label: "Cannot start with a period ( . )".to_owned(),
                        field_span: (
                            field_node.start_byte(),
                            field_node.end_byte() - field_node.start_byte(),
                        )
                            .into(),
                        error_span: (value_node.start_byte() + 1).into(),
                        help: format!(
                            "Use \x1b[0;32mpkgname=\"{}\"\x1b[0m instead",
                            &name[1..name.len()]
                        ),
                    });
                }
            }

            let check = |character: char| {
                character.is_ascii_alphabetic() && character.is_lowercase()
                    || character.is_ascii_digit()
                    || character == '@'
                    || character == '.'
                    || character == '_'
                    || character == '+'
                    || character == '-'
            };

            if !check(character) {
                return Err(FieldError {
                    field_label: "Can only contain lowercase, alphanumerics or @._+-".to_owned(),
                    field_span: (
                        field_node.start_byte(),
                        field_node.end_byte() - field_node.start_byte(),
                    )
                        .into(),
                    error_span: (value_node.start_byte() + 1 + index).into(),
                    help: format!("Use \x1b[0;32mpkgname=\"{}\"\x1b[0m instead", {
                        let mut name = name.to_owned();
                        name.retain(check);
                        name
                    }),
                });
            }
        }

        Ok(Self(name.to_string()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PkgverType {
    Variable(Pkgver),
    Function(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pkgver(String);

impl Pkgver {
    pub fn new(version: &str, field_node: &Node, value_node: &Node) -> Result<Self, FieldError> {
        for (index, character) in version.chars().enumerate() {
            if !(character.is_ascii_alphanumeric() || character == '.' || character == '_') {
                return Err(FieldError {
                    field_label: "Can only contain alphanumerics, periods and underscores"
                        .to_owned(),
                    field_span: (
                        field_node.start_byte(),
                        field_node.end_byte() - field_node.start_byte(),
                    )
                        .into(),
                    error_span: (value_node.start_byte() + 1 + index).into(),
                    help: "Remove the invalid characters".into(),
                });
            }
        }

        Ok(Self(version.into()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Maintainer {
    name: String,
    emails: Option<Vec<String>>,
}

impl Maintainer {
    // FIXME: Proptest
    pub fn new(maintainer: &str, field_node: &Node, value_node: &Node) -> Result<Self, FieldError> {
        let mut split: Vec<String> = maintainer
            .split_whitespace()
            .map(ToString::to_string)
            .collect();

        Ok(Self {
            name: match split.first() {
                Some(name) => name.trim().into(),
                None => {
                    return Err(FieldError {
                        field_label: "Needs a maintainer name".to_owned(),
                        field_span: (
                            field_node.start_byte(),
                            field_node.end_byte() - field_node.start_byte(),
                        )
                            .into(),
                        error_span: (value_node.start_byte() + 1).into(),
                        help: "Add a maintainer name. This is usually your GitHub username".into(),
                    });
                },
            },
            emails: {
                if split.len() > 1 {
                    let mut emails = vec![];
                    for email in &mut split[1..] {
                        if !email.ends_with('>') {
                            return Err(FieldError {
                                field_label: "Missing trailing >".to_owned(),
                                field_span: (
                                    field_node.start_byte(),
                                    field_node.end_byte() - field_node.start_byte(),
                                )
                                    .into(),
                                error_span: (value_node.end_byte() - 2).into(),
                                help: "Add a trailing > to the email address".into(),
                            });
                        }
                        let email = email.trim_matches(['<', '>'].as_ref());
                        if email.is_empty() {
                            return Err(FieldError {
                                field_label: "Email address cannot be empty".to_owned(),
                                field_span: (
                                    field_node.start_byte(),
                                    field_node.end_byte() - field_node.start_byte(),
                                )
                                    .into(),
                                error_span: (
                                    value_node.start_byte() + split[0].len() + 1,
                                    value_node.end_byte()
                                        - (value_node.start_byte() + split[0].len() + 2),
                                )
                                    .into(),
                                help: "Add the email address".into(),
                            });
                        }

                        emails.push((*email).to_string());
                    }

                    Some(emails)
                } else {
                    None
                }
            },
        })
    }
}

impl ToString for Maintainer {
    fn to_string(&self) -> String {
        match &self.emails {
            Some(emails) => {
                let mut maintainer_string = self.name.clone();

                for email in emails {
                    maintainer_string.push_str(&format!(" <{email}>"));
                }
                maintainer_string
            },
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub version_req: Option<VersionReq>,
}

impl Dependency {
    fn new(dependency: &str, field_node: &Node, value_node: &Node) -> Result<Self, FieldError> {
        let split: Vec<&str> = dependency.split(':').collect();

        let name = split[0].to_owned();

        if !name.is_ascii() {
            return Err(FieldError {
                field_label: "Name has to be valid ASCII".to_owned(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (
                    value_node.start_byte() + 1,
                    value_node.end_byte() - value_node.start_byte(),
                )
                    .into(),
                help: "Try romanizing your dependency name.".to_owned(),
            });
        }

        let version_req = match split.get(1) {
            Some(req) => match VersionReq::parse(req.trim()) {
                Ok(req) => Some(req),
                Err(error) => {
                    dbg!(req);
                    return Err(FieldError {
                        field_label: error.to_string(),
                        field_span: (
                            field_node.start_byte(),
                            field_node.end_byte() - field_node.start_byte(),
                        )
                            .into(),
                        error_span: (
                            value_node.start_byte() + 1 + name.len() + 2,
                            value_node.end_byte() - value_node.start_byte() - name.len() - 4,
                        )
                            .into(),
                        help: "The version requirements syntax is defined here: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html".into(),
                    });
                },
            },
            None => None,
        };

        Ok(Self { name, version_req })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct OptionalDependency {
    pub name: String,
    pub description: Option<String>,
}

impl OptionalDependency {
    fn new(
        optional_dependency: &str,
        field_node: &Node,
        value_node: &Node,
    ) -> Result<Self, FieldError> {
        // package:i386: desc

        // let Some(name, description) = optional_dependency.rsplit_once(":") else
        // };

        if optional_dependency.is_empty() {
            return Err(FieldError {
                field_label: "Cannot be empty".into(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (
                    value_node.start_byte(),
                    value_node.end_byte() - value_node.start_byte(),
                )
                    .into(),
                help: "Remove this empty field".into(),
            });
        }

        let (name, description) = match optional_dependency.rsplit_once(':') {
            Some((name, raw_description)) => {
                // l:d l: d
                // Remove the first leading space (` `) from the raw description, which is part
                // of the syntax
                let description = &raw_description[1..];
                let trim_start_description = description.trim_start();
                let trim_end_description = description.trim_end();
                let trimmed_description = description.trim();

                // Succeeds if the syntactic leading space wasn't present in the raw
                // description
                dbg!(description, raw_description);
                if raw_description.starts_with(' ')
                    && raw_description.chars().nth(1).unwrap() != ' '
                {
                    return Err(FieldError {
                        field_label: "The syntactic leading space is missing".to_owned(),
                        field_span: (
                            field_node.start_byte(),
                            field_node.end_byte() - field_node.start_byte(),
                        )
                            .into(),
                        error_span: (
                            value_node.start_byte() + 1 + name.len() + 2,
                            description.len() - trim_start_description.len(),
                        )
                            .into(),
                        help: format!(
                            "Use this instead: \x1b[0;32m\"{name}: {trimmed_description}\"\x1b[0m"
                        ),
                    });
                }

                // Check for leading spaces
                if trim_start_description != description {
                    return Err(FieldError {
                        field_label: "Extra leading spaces are invalid".to_owned(),
                        field_span: (
                            field_node.start_byte(),
                            field_node.end_byte() - field_node.start_byte(),
                        )
                            .into(),
                        error_span: (
                            value_node.start_byte() + 1 + name.len() + 2,
                            description.len() - trim_start_description.len(),
                        )
                            .into(),
                        help: format!(
                            "Use this instead: \x1b[0;32m\"{name}: {trimmed_description}\"\x1b[0m"
                        ),
                    });
                }

                // Check for trailing spaces
                if description.trim_end() != description {
                    return Err(FieldError {
                        field_label: "Trailing spaces are invalid".to_owned(),
                        field_span: (
                            field_node.start_byte(),
                            field_node.end_byte() - field_node.start_byte(),
                        )
                            .into(),

                        error_span: (
                            value_node.start_byte()
                                + 1
                                + name.len()
                                + 2
                                + trimmed_description.len(),
                            description.len() - trim_end_description.len(),
                        )
                            .into(),
                        help: format!(
                            "Use this instead: \x1b[0;32m\"{name}: {trimmed_description}\"\x1b[0m"
                        ),
                    });
                }

                (name.to_owned(), Some(description.to_owned()))
            },
            None => (optional_dependency.to_owned(), None),
        };

        Ok(Self { name, description })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PPA {
    pub owner: String,
    pub package: String,
}

impl PPA {
    pub fn new(ppa: &str, field_node: &Node, value_node: &Node) -> Result<Self, FieldError> {
        let split: Vec<&str> = ppa.split('/').collect();

        if split.len() == 1 {
            return Err(FieldError {
                field_label: "Must contain the PPA in the format: owner/package".to_owned(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (
                    value_node.start_byte() + 1,
                    value_node.end_byte() - (value_node.start_byte() + 2),
                )
                    .into(),
                help: "Add the PPA in proper format. Example: kelleyk/emacs".into(),
            });
        }

        Ok(Self {
            owner: split[0].into(),
            package: split[1].into(),
        })
    }
}

impl ToString for PPA {
    fn to_string(&self) -> String { format!("{}/{}", self.owner, self.package) }
}

#[derive(Debug, PartialEq, Eq, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum RepologyStatus {
    Newest,
    Devel,
    Unique,
    Outdated,
    Legacy,
    Rolling,
    NoScheme,
    Incorrect,
    Untrusted,
    Ignored,
}

#[derive(Debug, PartialEq, Eq, Display)]
#[strum(serialize_all = "lowercase")]
pub enum RepologyFilter {
    Project(String),
    Repo(String),
    SubRepo(String),
    Name(String),
    SrcName(String),
    BinName(String),
    VisibleName(String),
    Version(String),
    OrigVersion(String),
    Status(RepologyStatus),
    Summary(String),
}

impl RepologyFilter {
    #[allow(clippy::too_many_lines)]
    fn new(
        repology_filter: &str,
        field_node: &Node,
        value_node: &Node,
    ) -> Result<Self, FieldError> {
        let split: Vec<&str> = repology_filter.split(':').collect();

        if split.len() != 2 {
            return Err(FieldError {
                field_label: "Must contain the repology filter in the format: `filter: value`"
                    .into(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (
                    value_node.start_byte() + 1,
                    value_node.end_byte() - value_node.start_byte() - 2,
                )
                    .into(),
                help: "Add the repology filter in proper format. Example: `project: emacs`".into(),
            });
        }

        // Verify the filter is properly formatted
        if split[0].chars().any(char::is_whitespace) {
            return Err(FieldError {
                field_label: "Filter must not contain whitespaces".into(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (value_node.start_byte() + 1, split[0].len()).into(),
                help: format!(
                    "Maybe you meant this instead: `{}`",
                    split[0].replace(' ', "")
                ),
            });
        }

        // Verify that the value is properly formatted
        if !split[1].starts_with(' ') {
            return Err(FieldError {
                field_label: "Value must start with a space".into(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (value_node.start_byte() + split[0].len() + 2, 1).into(),
                help: format!("Use this: `{}: {}`", split[0], split[1].trim()),
            });
        }

        let Some(value) = split[1].get(1..) else {
            return Err(FieldError {
                field_label: "Value cannot be empty".into(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (value_node.start_byte() + split[0].len() + 2, 1).into(),
                help: "Add the repology filter in proper format. Example: `project: emacs`".into(),
            });
        };

        let value = value.to_owned();

        if value.trim().is_empty() {
            return Err(FieldError {
                field_label: "Value cannot be empty".into(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (
                    value_node.start_byte() + split[0].len() + 2,
                    value.len() + 1,
                )
                    .into(),
                help: "Add the repology filter in proper format. Example: `project: emacs`".into(),
            });
        }

        if value.chars().any(char::is_whitespace) {
            return Err(FieldError {
                field_label: "Value must not contain whitespaces".into(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (value_node.start_byte() + split[0].len() + 2, split[1].len()).into(),
                help: format!(
                    "Use this: `{}: {}`",
                    split[0],
                    split[1]
                        .chars()
                        .filter(|c| c.is_whitespace())
                        .collect::<String>()
                ),
            });
        }

        let filter = match split[0] {
            "project" => Self::Project(value),
            "repo" => Self::Repo(value),
            "subrepo" => Self::SubRepo(value),
            "name" => Self::Name(value),
            "srcname" => Self::SrcName(value),
            "binname" => Self::BinName(value),
            "visiblename" => Self::VisibleName(value),
            "version" => Self::Version(value),
            "origversion" => Self::OrigVersion(value),
            "status" => Self::Status(match split[1].parse() {
                Ok(status) => status,
                Err(_) => {
                    return Err(FieldError {
                        field_label: "Invalid status".into(),
                        field_span: (
                            field_node.start_byte(),
                            field_node.end_byte() - field_node.start_byte(),
                        )
                            .into(),
                        error_span: (value_node.start_byte() + split[0].len() + 2, split[1].len())
                            .into(),
                        help: "Use one of `newest`, `devel`, `unique`, `outdated`, `legacy`, \
                               `rolling`, `noscheme`, `incorrect`, `untrusted`, `ignored`"
                            .into(),
                    })
                },
            }),
            "summary" => Self::Summary(value),
            _ => {
                return Err(FieldError {
                    field_label: "Invalid filter".into(),
                    field_span: (
                        field_node.start_byte(),
                        field_node.end_byte() - field_node.start_byte(),
                    )
                        .into(),
                    error_span: (value_node.start_byte() + 1, split[0].len()).into(),
                    help: "Use one of `project`, `repo`, `subrepo`, `name`, `srcname`, `binname`, \
                           `visiblename`, `version`, `origversion`, `status`, `summary`"
                        .to_owned(),
                });
            },
        };

        Ok(filter)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum GitFragment {
    Branch(String),
    Commit(String),
    Tag(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum GitSource {
    File(PathBuf),
    HTTPS(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum SourceLink {
    HTTPS(String),
    Git {
        source_type: GitSource,
        fragment: Option<GitFragment>,
        query_signed: bool,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Source {
    pub name: Option<String>,
    pub link: SourceLink,
    pub repology: bool,
}

impl Source {
    #[allow(clippy::too_many_lines)]
    fn new(source: &str, field_node: &Node, value_node: &Node) -> Result<Self, FieldError> {
        let field_span: SourceSpan = (
            field_node.start_byte(),
            field_node.end_byte() - field_node.start_byte(),
        )
            .into();
        let split: Vec<&str> = source.split("::").collect();

        let mut raw_repology = None;
        let mut name = None;
        let mut link = String::new();
        let mut repology = false;
        match split.len() {
            1 => {
                link = split[0].to_owned();
            },
            2 => {
                if split[0].contains("://") {
                    link = split[0].to_owned();
                    raw_repology = Some(split[1]);
                } else {
                    name = Some(split[0].to_owned());
                    link = split[1].to_owned();
                }
            },
            3 => {
                name = Some(split[0].to_owned());
                link = split[1].to_owned();
                raw_repology = Some(split[2]);

                repology = true;
            },
            _ => todo!(),
        };

        // Repology checks
        if let Some(raw_repology) = raw_repology {
            if raw_repology != "repology" {
                if raw_repology.chars().any(char::is_whitespace) {
                    let whitespace_characters = raw_repology
                        .chars()
                        .skip_while(|c| !c.is_whitespace())
                        .take_while(|c| c.is_whitespace())
                        .count();

                    let characters_until_whitespaces = raw_repology
                        .chars()
                        .take_while(|c| !c.is_whitespace())
                        .count();

                    return Err(FieldError {
                        field_label: "Invalid whitespaces".into(),
                        field_span,
                        error_span: (
                            value_node.start_byte()
                                + ((source.len() - raw_repology.len()) + 1)
                                + characters_until_whitespaces,
                            whitespace_characters,
                        )
                            .into(),
                        help: format!(
                            "Remove the invalid whitespaces. You probably meant this instead: \
                             `{}::{}::repology`",
                            split[0], split[1]
                        ),
                    });
                }

                return Err(FieldError {
                    field_label: if raw_repology.is_empty() {
                        "Missing repology key".into()
                    } else {
                        "Invalid key".into()
                    },
                    field_span,
                    error_span: (
                        value_node.start_byte() + (source.len() - raw_repology.len() + 1),
                        raw_repology.len(),
                    )
                        .into(),
                    help: format!(
                        "Maybe you meant to use the repology key, like this: `{}::{}::repology`",
                        split[0], split[1]
                    ),
                });
            }
            repology = true;
        }

        // Link checks
        let whitespace_characters = link
            .chars()
            .skip_while(|c| !c.is_whitespace())
            .take_while(|c| c.is_whitespace())
            .count();

        if whitespace_characters > 0 {
            let characters_until_whitespaces =
                link.chars().take_while(|c| !c.is_whitespace()).count();

            return Err(FieldError {
                field_label: "Invalid whitespaces".into(),
                field_span,
                error_span: (
                    value_node.start_byte()
                        + 1
                        + name.map_or(0, |name| name.len() + 2)
                        + characters_until_whitespaces,
                    whitespace_characters,
                )
                    .into(),
                help: format!(
                    "Remove the invalid whitespaces. You probably meant this instead: `{}`",
                    link.chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>()
                ),
            });
        }

        let protocol_split = link.split("://").collect::<Vec<_>>();

        if protocol_split.len() != 2 {
            return Err(FieldError {
                field_label: "No protocol specified".into(),
                field_span: (
                    field_node.start_byte(),
                    field_node.end_byte() - field_node.start_byte(),
                )
                    .into(),
                error_span: (
                    value_node.start_byte(),
                    value_node.end_byte() - value_node.start_byte(),
                )
                    .into(),
                help: "Use one of `https`, `git`, `magnet`, `ftp`".into(),
            });
        }

        let (protocol, link_without_protocol) = (protocol_split[0], protocol_split[1]);

        let link = link_without_protocol
            .find(['#', '?'])
            .map_or(link_without_protocol, |i| &link_without_protocol[..i]);

        let protocol = match protocol {
            "https" => SourceLink::HTTPS(link.to_owned()),
            git if git.starts_with("git") => SourceLink::Git {
                source_type: {
                    let split: Vec<_> = protocol.split('+').collect();

                    if split.len() != 2 {
                        return Err(FieldError {
                            field_label: "No git protocol
                    specified"
                                .into(),
                            field_span: (
                                field_node.start_byte(),
                                field_node.end_byte() - field_node.start_byte(),
                            )
                                .into(),
                            error_span: (
                                value_node.start_byte(),
                                value_node.end_byte() - value_node.start_byte(),
                            )
                                .into(),
                            help: "Specify a git protocol like:
                    `git+https` or `git+file`"
                                .into(),
                        });
                    }

                    match split[1] {
                        "https" => GitSource::HTTPS(link.to_owned()),
                        "file" => {
                            let repo_dir = PathBuf::from(link);
                            if !repo_dir.exists() {
                                todo!("Repository doesn't exist");
                            }
                            if !repo_dir.is_dir() {
                                todo!("Repository isn't a directory");
                            }
                            GitSource::File(repo_dir)
                        },
                        _ => {
                            return Err(FieldError {
                                field_label: "Invalid git
                    protocol"
                                    .into(),
                                field_span: (
                                    field_node.start_byte(),
                                    field_node.end_byte() - field_node.start_byte(),
                                )
                                    .into(),
                                error_span: (
                                    value_node.start_byte(),
                                    value_node.end_byte() - value_node.start_byte(),
                                )
                                    .into(),
                                help: "Specify a git protocol like:
                    `git+https` or `git+file`"
                                    .into(),
                            });
                        },
                    }
                },
                fragment: {
                    match link_without_protocol.matches('#').count() {
                        2.. => todo!("Invalid number of #"),
                        1 => {
                            let fragment = &link_without_protocol
                                .get(
                                    link_without_protocol.find('#').unwrap()
                                        ..link_without_protocol
                                            .find('?')
                                            .unwrap_or(link_without_protocol.len() - 1),
                                )
                                .unwrap_or_else(|| todo!("Invalid sequence, ? before #"));

                            let split: Vec<&str> = fragment.split('=').collect();

                            if split.len() > 2 {
                                todo!("Invalid number of =");
                            }

                            let (fragment_type, value) = (&split[0][1..], split[1].to_owned());

                            match fragment_type {
                                "branch" => Some(GitFragment::Branch(value)),
                                "tag" => Some(GitFragment::Tag(value)),
                                "commit" => Some(GitFragment::Commit(value)),
                                _ => todo!("Invalid fragment"),
                            }
                        },
                        0 => None,
                        _ => unreachable!("Broke math"),
                    }
                },
                query_signed: {
                    match link_without_protocol.matches('?').count() {
                        2.. => todo!("Invalid number of ?"),
                        1 => {
                            let query =
                                &link_without_protocol[link_without_protocol.find('?').unwrap() + 1
                                    ..=std::cmp::max(
                                        link_without_protocol.find('#').unwrap_or(0),
                                        link_without_protocol.len() - 1,
                                    )];

                            match query {
                                "signed" => true,
                                _ => todo!("Invalid query"),
                            }
                        },
                        0 => false,
                        _ => unreachable!("Broke math"),
                    }
                },
            },
            _ => {
                return Err(FieldError {
                    field_label: "Invalid protocol".into(),
                    field_span: (
                        field_node.start_byte(),
                        field_node.end_byte() - field_node.start_byte(),
                    )
                        .into(),
                    error_span: (
                        value_node.start_byte(),
                        value_node.end_byte() - value_node.start_byte(),
                    )
                        .into(),
                    help: "Specify a git protocol like: `https`, `git+https`, `git+file`, \
                           `magnet` or `ftp`"
                        .into(),
                });
            },
        };

        match &protocol {
            SourceLink::Git {
                source_type: GitSource::File(_),
                fragment: _,
                query_signed: _,
            } => {},
            _ => {
                if !Regex::new(
                    r"(www\.)?[-a-zA-Z0-9@:%._\+~#=]{2,256}\.[a-z]{2,6}\b([-a-zA-Z0-9@:%_\+.~#?&//=]*)",
                )
                .unwrap()
                .is_match(link_without_protocol)
                {
                    todo!("Invalid URL SIR");
                }
            },
        }

        Ok(Self {
            repology,
            name,
            link: protocol,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct PacBuild {
    pub pkgname: Vec<Pkgname>,
    pub pkgver: PkgverType,
    pub epoch: Option<u32>,
    pub pkgdesc: Option<String>,
    pub url: Option<String>,
    pub license: Option<Expression>,
    pub custom_variables: Option<HashMap<String, String>>,

    pub arch: Vec<String>,
    pub maintainer: Option<Vec<Maintainer>>,
    pub noextract: Option<Vec<String>>,
    pub sha256sums: Option<HashMap<String, Vec<Option<String>>>>,
    pub sha348sums: Option<HashMap<String, Vec<Option<String>>>>,
    pub sha512sums: Option<HashMap<String, Vec<Option<String>>>>,
    pub b2sums: Option<HashMap<String, Vec<Option<String>>>>,
    pub depends: Option<Vec<Dependency>>,
    pub optdepends: Option<Vec<OptionalDependency>>,
    pub ppa: Option<Vec<PPA>>,
    pub repology: Option<Vec<RepologyFilter>>,
    pub sources: Vec<Source>,

    pub prepare: Option<String>,
    pub build: Option<String>,
    pub check: Option<String>,
    pub package: Option<String>,
    pub pre_install: Option<String>,
    pub post_install: Option<String>,
    pub pre_upgrade: Option<String>,
    pub post_upgrade: Option<String>,
    pub pre_remove: Option<String>,
    pub post_remove: Option<String>,
    pub custom_functions: Option<HashMap<String, String>>,
}

impl PacBuild {
    fn cleanup_rawstring(raw_string: &str) -> &str {
        let len = raw_string.len();
        if len <= 2 {
            ""
        } else {
            &raw_string[1..len - 1]
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn from_source(source_code: &str) -> Result<Self, ParseError> {
        let mut pkgname: Option<Vec<Pkgname>> = None;
        let mut pkgver: Option<PkgverType> = None;
        let mut epoch: Option<u32> = None;
        let mut pkgdesc: Option<String> = None;
        let mut url: Option<String> = None;
        let mut license: Option<Expression> = None;
        let mut custom_variables: Option<HashMap<String, String>> = None;

        let mut arch: Option<Vec<String>> = None;
        let mut maintainer: Option<Vec<Maintainer>> = None;
        let mut noextract: Option<Vec<String>> = None;
        let mut sha256sums: Option<HashMap<String, Vec<Option<String>>>> = None;
        let mut sha348sums: Option<HashMap<String, Vec<Option<String>>>> = None;
        let mut sha512sums: Option<HashMap<String, Vec<Option<String>>>> = None;
        let mut b2sums: Option<HashMap<String, Vec<Option<String>>>> = None;
        let mut depends: Option<Vec<Dependency>> = None;
        let mut optdepends: Option<Vec<OptionalDependency>> = None;
        let mut ppa: Option<Vec<PPA>> = None;
        let mut repology: Option<Vec<RepologyFilter>> = None;
        let mut sources: Option<Vec<Source>> = None;

        let mut prepare: Option<String> = None;
        let mut build: Option<String> = None;
        let mut check: Option<String> = None;
        let mut package: Option<String> = None;
        let mut pre_install: Option<String> = None;
        let mut post_install: Option<String> = None;
        let mut pre_upgrade: Option<String> = None;
        let mut post_upgrade: Option<String> = None;
        let mut pre_remove: Option<String> = None;
        let mut post_remove: Option<String> = None;
        let mut custom_functions: Option<HashMap<String, String>> = None;

        #[allow(clippy::similar_names)]
        let sourced_code = Command::new("bash")
            .args([
                "-c",
                &format!(r#"{source_code}; SOURCED_CODE="$(declare -p | cut -d ' ' -f 3-)"; TAIL="$(echo "$SOURCED_CODE" | grep -m 1 -n '_=\"\"' | cut -d ':' -f 1)"; echo "$SOURCED_CODE" | tail -n +$(($TAIL + 1)); declare -f"#),
            ])
            .output()
            .unwrap();

        // dbg!(String::from_utf8(sourced_code.stderr.clone()).unwrap());

        let mut errors: Vec<Report> = vec![];

        if !sourced_code.status.success() {
            errors.push(Report::new_boxed(Box::new(BadSyntax {})));
        }

        let mut parser = Parser::new();

        parser.set_language(tree_sitter_bash::language()).unwrap();

        let Some(tree) = parser.parse(sourced_code.stdout.clone(), None) else { {
                return Err(ParseError {
                    input: source_code.into(),
                    related: vec![Report::new_boxed(Box::new(BadSyntax {}))],
                })
            } };

        let mut query = QueryCursor::new();

        for (query_match, index) in query.captures(
            &Query::new(
                tree_sitter_bash::language(),
                "(program (variable_assignment
                    name:  (variable_name) @variable_name
                    value: [
                    (array (concatenation (
                        (word)
                        (word)
                        [(raw_string) (string) (word)] @assoc_array
                    )))
                    [(raw_string) (string) (word)] @value]))
                (function_definition
                    name: (word) @function_name
                ) @function",
            )
            .unwrap(),
            tree.root_node(),
            |_| sourced_code.stdout.clone(),
        ) {
            if index == 1 {
                for capture in query_match.captures {
                    match capture.index {
                        // Variable name
                        0 => {
                            let field_node = query_match.captures[0].node;
                            let name = field_node.utf8_text(&sourced_code.stdout).unwrap();

                            let index = query_match.captures[1].index;

                            match index {
                                // It's a normal variable.
                                2 => {
                                    let value_node = query_match.captures[1].node;
                                    let value = Self::cleanup_rawstring(
                                        value_node.utf8_text(&sourced_code.stdout).unwrap(),
                                    );

                                    match name {
                                        "pkgname" => {
                                            match Pkgname::new(value, &field_node, &value_node) {
                                                Ok(name) => pkgname = Some(vec![name]),
                                                Err(error) => {
                                                    errors.push(Report::new_boxed(Box::new(error)));
                                                },
                                            };
                                        },
                                        "pkgver" => {
                                            match Pkgver::new(value, &field_node, &value_node) {
                                                Ok(ver) => pkgver = Some(PkgverType::Variable(ver)),
                                                Err(error) => {
                                                    errors.push(Report::new_boxed(Box::new(error)));
                                                },
                                            };
                                        },
                                        "epoch" => {
                                            match value.parse() {
                                                Ok(value) => epoch = Some(value),
                                                Err(_error) => errors.push(Report::new_boxed(
                                                    Box::new(FieldError {
                                                        field_label: "Can only be a non-negative \
                                                                      integer"
                                                            .to_owned(),
                                                        field_span: (
                                                            field_node.start_byte(),
                                                            field_node.end_byte()
                                                                - field_node.start_byte(),
                                                        )
                                                            .into(),
                                                        error_span: (
                                                            value_node.start_byte() + 1,
                                                            value_node.end_byte()
                                                                - value_node.start_byte()
                                                                - 2,
                                                        )
                                                            .into(),
                                                        help: "Use a non-negative epoch".into(),
                                                    }),
                                                )),
                                            };
                                        },
                                        "pkgdesc" => {
                                            pkgdesc = Some(value.into());
                                        },
                                        "license" => {
                                            match Expression::parse(value)
                                                .into_diagnostic()
                                                .context("Invalid license field")
                                            {
                                                Ok(expr) => license = Some(expr),
                                                Err(error) => errors.push(error),
                                            };
                                        },
                                        "url" => {
                                            url = Some(value.into());
                                        },
                                        _ => match &mut custom_variables {
                                            Some(custom_variables) => {
                                                custom_variables.insert(name.into(), value.into());
                                            },
                                            None => {
                                                custom_variables = Some(HashMap::from([(
                                                    name.into(),
                                                    value.into(),
                                                )]));
                                            },
                                        },
                                    }
                                },
                                // Array
                                1 => {
                                    let value_node = query_match.captures[1].node;
                                    let value = Self::cleanup_rawstring(
                                        value_node.utf8_text(&sourced_code.stdout).unwrap(),
                                    );

                                    match name {
                                        "arch" => match &mut arch {
                                            Some(arch) => arch.push(value.into()),
                                            None => arch = Some(vec![value.into()]),
                                        },
                                        "maintainer" => match &mut maintainer {
                                            Some(maintainer_vec) => match Maintainer::new(
                                                value,
                                                &field_node,
                                                &value_node,
                                            ) {
                                                Ok(maintainer) => maintainer_vec.push(maintainer),
                                                Err(error) => {
                                                    errors.push(Report::new_boxed(Box::new(error)));
                                                },
                                            },
                                            None => {
                                                match Maintainer::new(
                                                    value,
                                                    &field_node,
                                                    &value_node,
                                                ) {
                                                    Ok(a_maintainer) => {
                                                        maintainer = Some(vec![a_maintainer]);
                                                    },
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                };
                                            },
                                        },
                                        "noextract" => match &mut noextract {
                                            Some(noextract) => noextract.push(value.into()),
                                            None => noextract = Some(vec![value.into()]),
                                        },
                                        shasum if shasum.starts_with("sha256sums") => {
                                            let checksum_arch =
                                                shasum.strip_prefix("sha256sums_").unwrap_or("any");

                                            match &mut sha256sums {
                                                Some(sha256sums) => {
                                                    match sha256sums.get_mut(checksum_arch) {
                                                        Some(hashes) => {
                                                            hashes.push(if value == "SKIP" {
                                                                None
                                                            } else {
                                                                Some(value.into())
                                                            });
                                                        },
                                                        None => {
                                                            sha256sums.insert(
                                                                checksum_arch.into(),
                                                                vec![if value == "SKIP" {
                                                                    None
                                                                } else {
                                                                    Some(value.into())
                                                                }],
                                                            );
                                                        },
                                                    };
                                                },
                                                None => {
                                                    sha256sums = Some(HashMap::from([(
                                                        checksum_arch.into(),
                                                        vec![if value == "SKIP" {
                                                            None
                                                        } else {
                                                            Some(value.into())
                                                        }],
                                                    )]));
                                                },
                                            }
                                        },
                                        shasum if shasum.starts_with("sha348sums") => {
                                            let checksum_arch =
                                                shasum.strip_prefix("sha348sums_").unwrap_or("any");

                                            match &mut sha348sums {
                                                Some(sha348sums) => {
                                                    match sha348sums.get_mut(checksum_arch) {
                                                        Some(hashes) => {
                                                            hashes.push(if value == "SKIP" {
                                                                None
                                                            } else {
                                                                Some(value.into())
                                                            });
                                                        },
                                                        None => {
                                                            sha348sums.insert(
                                                                checksum_arch.into(),
                                                                vec![if value == "SKIP" {
                                                                    None
                                                                } else {
                                                                    Some(value.into())
                                                                }],
                                                            );
                                                        },
                                                    };
                                                },
                                                None => {
                                                    sha348sums = Some(HashMap::from([(
                                                        checksum_arch.into(),
                                                        vec![if value == "SKIP" {
                                                            None
                                                        } else {
                                                            Some(value.into())
                                                        }],
                                                    )]));
                                                },
                                            }
                                        },
                                        shasum if shasum.starts_with("sha512sums") => {
                                            let checksum_arch =
                                                shasum.strip_prefix("sha512sums_").unwrap_or("any");

                                            match &mut sha512sums {
                                                Some(sha512sums) => {
                                                    match sha512sums.get_mut(checksum_arch) {
                                                        Some(hashes) => {
                                                            hashes.push(if value == "SKIP" {
                                                                None
                                                            } else {
                                                                Some(value.into())
                                                            });
                                                        },
                                                        None => {
                                                            sha512sums.insert(
                                                                checksum_arch.into(),
                                                                vec![if value == "SKIP" {
                                                                    None
                                                                } else {
                                                                    Some(value.into())
                                                                }],
                                                            );
                                                        },
                                                    };
                                                },
                                                None => {
                                                    sha512sums = Some(HashMap::from([(
                                                        checksum_arch.into(),
                                                        vec![if value == "SKIP" {
                                                            None
                                                        } else {
                                                            Some(value.into())
                                                        }],
                                                    )]));
                                                },
                                            }
                                        },
                                        shasum if shasum.starts_with("b2sums") => {
                                            let checksum_arch =
                                                shasum.strip_prefix("b2sums_").unwrap_or("any");

                                            match &mut b2sums {
                                                Some(b2sums) => {
                                                    match b2sums.get_mut(checksum_arch) {
                                                        Some(hashes) => {
                                                            hashes.push(if value == "SKIP" {
                                                                None
                                                            } else {
                                                                Some(value.into())
                                                            });
                                                        },
                                                        None => {
                                                            b2sums.insert(
                                                                checksum_arch.into(),
                                                                vec![if value == "SKIP" {
                                                                    None
                                                                } else {
                                                                    Some(value.into())
                                                                }],
                                                            );
                                                        },
                                                    };
                                                },
                                                None => {
                                                    b2sums = Some(HashMap::from([(
                                                        checksum_arch.into(),
                                                        vec![if value == "SKIP" {
                                                            None
                                                        } else {
                                                            Some(value.into())
                                                        }],
                                                    )]));
                                                },
                                            }
                                        },
                                        "depends" => match &mut depends {
                                            Some(depends_vec) => {
                                                match Dependency::new(
                                                    value,
                                                    &field_node,
                                                    &value_node,
                                                ) {
                                                    Ok(dependency) => depends_vec.push(dependency),
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                }
                                            },
                                            None => {
                                                match Dependency::new(
                                                    value,
                                                    &field_node,
                                                    &value_node,
                                                ) {
                                                    Ok(dependency) => {
                                                        depends = Some(vec![dependency]);
                                                    },
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                }
                                            },
                                        },
                                        "optdepends" => match &mut optdepends {
                                            Some(optdepends_vec) => {
                                                match OptionalDependency::new(
                                                    value,
                                                    &field_node,
                                                    &value_node,
                                                ) {
                                                    Ok(optional_dependency) => {
                                                        optdepends_vec.push(optional_dependency);
                                                    },
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                }
                                            },
                                            None => {
                                                match OptionalDependency::new(
                                                    value,
                                                    &field_node,
                                                    &value_node,
                                                ) {
                                                    Ok(optional_dependency) => {
                                                        optdepends =
                                                            Some(vec![optional_dependency]);
                                                    },
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                }
                                            },
                                        },
                                        "ppa" => match &mut ppa {
                                            Some(ppa_vec) => {
                                                match PPA::new(value, &field_node, &value_node) {
                                                    Ok(ppa) => ppa_vec.push(ppa),
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                }
                                            },
                                            None => {
                                                match PPA::new(value, &field_node, &value_node) {
                                                    Ok(a_ppa) => ppa = Some(vec![a_ppa]),
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                };
                                            },
                                        },
                                        "repology" => match &mut repology {
                                            Some(repology_vec) => {
                                                match RepologyFilter::new(
                                                    value,
                                                    &field_node,
                                                    &value_node,
                                                ) {
                                                    Ok(repology_filter) => {
                                                        repology_vec.push(repology_filter);
                                                    },
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                }
                                            },
                                            None => {
                                                match RepologyFilter::new(
                                                    value,
                                                    &field_node,
                                                    &value_node,
                                                ) {
                                                    Ok(repology_filter) => {
                                                        repology = Some(vec![repology_filter]);
                                                    },
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                };
                                            },
                                        },
                                        "sources" => match &mut sources {
                                            Some(sources_vec) => {
                                                match Source::new(value, &field_node, &value_node) {
                                                    Ok(source) => sources_vec.push(source),
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                }
                                            },
                                            None => {
                                                match Source::new(value, &field_node, &value_node) {
                                                    Ok(source) => sources = Some(vec![source]),
                                                    Err(error) => errors
                                                        .push(Report::new_boxed(Box::new(error))),
                                                };
                                            },
                                        },
                                        _ => {},
                                    }
                                },
                                _ => {},
                            }
                        },
                        // Function definition
                        4 => {
                            let name = query_match.captures[1]
                                .node
                                .utf8_text(&sourced_code.stdout)
                                .unwrap();
                            let function = query_match.captures[0]
                                .node
                                .utf8_text(&sourced_code.stdout)
                                .unwrap();

                            match name {
                                "prepare" => prepare = Some(function.into()),
                                "build" => build = Some(function.into()),
                                "check" => check = Some(function.into()),
                                "package" => package = Some(function.into()),
                                "pre_install" => pre_install = Some(function.into()),
                                "post_install" => post_install = Some(function.into()),
                                "pre_upgrade" => pre_upgrade = Some(function.into()),
                                "post_upgrade" => post_upgrade = Some(function.into()),
                                "pre_remove" => pre_remove = Some(function.into()),
                                "post_remove" => post_remove = Some(function.into()),
                                _ => match &mut custom_functions {
                                    Some(custom_functions) => {
                                        custom_functions.insert(name.into(), function.into());
                                    },
                                    None => {
                                        custom_functions =
                                            Some(HashMap::from([(name.into(), function.into())]));
                                    },
                                },
                            };
                        },
                        _ => {},
                    };
                }
            }
        }

        if !errors.is_empty() {
            return Err(ParseError {
                input: String::from_utf8(sourced_code.stdout).unwrap(),
                related: errors,
            });
        }

        let Some(pkgname) = pkgname else {
            return Err(ParseError {
                input: String::from_utf8(sourced_code.stdout).unwrap(),
                related: {
                    errors.push(Report::new_boxed(Box::new(MissingField {
                        label: "pkgname is missing",
                    })));
                    errors
                },
            });
        };

        let Some(pkgver) = pkgver else {
            return Err(ParseError {
                input: String::from_utf8(sourced_code.stdout).unwrap(),
                related: {
                    errors.push(Report::new_boxed(Box::new(MissingField {
                        label: "pkgver is missing",
                    })));
                    errors
                },
            });
        };

        let Some(arch) = arch else {
            return Err(ParseError {
                input: String::from_utf8(sourced_code.stdout).unwrap(),
                related: {
                    errors.push(Report::new_boxed(Box::new(MissingField {
                        label: "pkgver is missing",
                    })));
                    errors
                },
            });
        };

        let Some(sources) = sources else {
            return Err(ParseError {
                input: String::from_utf8(sourced_code.stdout).unwrap(),
            related: { errors.push(Report::new_boxed(Box::new(MissingField { label: "source is missing"}))); errors},
            });
        };

        // TODO: Possibly check if checksum and sources lengths match

        let pkgbuild = Self {
            pkgname,
            pkgver,
            epoch,
            pkgdesc,
            url,
            license,
            custom_variables,
            arch,
            maintainer,
            noextract,
            sha256sums,
            sha348sums,
            sha512sums,
            b2sums,
            depends,
            optdepends,
            ppa,
            repology,
            sources,
            prepare,
            build,
            check,
            package,
            pre_install,
            post_install,
            pre_upgrade,
            post_upgrade,
            pre_remove,
            post_remove,
            custom_functions,
        };

        Ok(pkgbuild)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn test_pkgname(name in r#"[a-z0-9@_+]+[a-z0-9@._+-]+"#) {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_bash::language()).unwrap();
            let tree = parser.parse(b"test", None).unwrap();
            let parent = tree.root_node();

            let pkgname = Pkgname::new(&name, &parent, &parent).unwrap();
            assert_eq!(pkgname.0, name);
        }

        #[test]
        fn test_invalid_pkgname(name in r"[.-][^a-z0-9@._+-]+") {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_bash::language()).unwrap();
            let tree = parser.parse(b"test", None).unwrap();
            let parent = tree.root_node();

            let result = Pkgname::new(&name, &parent, &parent);
            assert!(result.is_err());
        }

        #[test]
        fn test_pkgver(version in r"[a-zA-Z0-9._]+") {
            let mut parser = Parser::new();

            parser.set_language(tree_sitter_bash::language()).unwrap();
            let tree = parser.parse(b"test", None).unwrap();
            let parent = tree.root_node();

            let pkgver = Pkgver::new(&version, &parent, &parent).unwrap();
            assert_eq!(pkgver.0, version);
        }

        #[test]
        fn test_invalid_pkgver(version in r"[^a-zA-Z0-9._]") {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_bash::language()).unwrap();
            let tree = parser.parse(b"test", None).unwrap();
            let parent = tree.root_node();

            let result = Pkgver::new(&version, &parent, &parent);
            assert!(result.is_err());
        }

        #[test]
        fn test_dependency(name in r#"[\x00-\x7F&&[^:]]+"#, version_req in r#"(?:(>=|<=|>|<|=|\^|~))?((0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})(?:-((?:0|[1-9][0-9]*|[0-9]*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9][0-9]*|[0-9]*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?)(?:, (?:(>=|<=|>|<|=|\^|~))?((0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})(?:-((?:0|[1-9][0-9]*|[0-9]*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9][0-9]*|[0-9]*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?)){0,31}"#) {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_bash::language()).unwrap();
            let tree = parser.parse(b"test", None).unwrap();
            let parent = tree.root_node();

            let dependency_without_version_req = Dependency::new(&name, &parent, &parent).unwrap();
            assert_eq!(dependency_without_version_req.name, name);
            assert_eq!(dependency_without_version_req.version_req, None);

            let dependency_without_version_req = Dependency::new(&(name.clone() + ": " + &version_req), &parent, &parent).unwrap();
            assert_eq!(dependency_without_version_req.name, name);
            assert_eq!(dependency_without_version_req.version_req, Some(VersionReq::parse(&version_req).unwrap()));
        }


        // #[test]
        // fn test_invalid_dependency(name in r#"[^\x00-\x7F]*"#, version_req in r#"[^(?:(>=|<=|>|<|=|\^|~))?((0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})(?:-((?:0|[1-9][0-9]*|[0-9]*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9][0-9]*|[0-9]*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?)(?:, (?:(>=|<=|>|<|=|\^|~))?((0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})(?:-((?:0|[1-9][0-9]*|[0-9]*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9][0-9]*|[0-9]*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?)){0,31}]"#) {
        //     let mut parser = Parser::new();
        //     parser.set_language(tree_sitter_bash::language()).unwrap();
        //     let tree = parser.parse(b"test", None).unwrap();
        //     let parent = tree.root_node();

        //     assert!(Dependency::new(&name, &parent, &parent).is_err());

        //     let dependency_without_version_req = Dependency::new(&(name.to_owned() + ": " + &version_req), &parent, &parent).unwrap();
        //     assert_eq!(dependency_without_version_req.name, name);
        //     assert_eq!(dependency_without_version_req.version_req, Some(VersionReq::parse(&version_req).unwrap()));
        // }

        // #[test]
        // fn test_maintainer(name in r"\S.*\S", email in r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+") {
        //     let mut parser = Parser::new();
        //     parser.set_language(tree_sitter_bash::language()).unwrap();
        //     let tree = parser.parse(b"test", None).unwrap();
        //     let parent = tree.root_node();


        //     let maintainer_without_email = Maintainer::new(&name, &parent, &parent).unwrap();
        //     assert_eq!(maintainer_without_email.name, name);
        // }

        #[test]
        fn test_repology(value in r"[^\s:]+") {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_bash::language()).unwrap();
            let tree = parser.parse(b"test", None).unwrap();
            let parent = tree.root_node();

            let repology_filter = RepologyFilter::new(&format!("name: {value}"), &parent, &parent).unwrap();
            if let RepologyFilter::Name(name) = repology_filter {
                assert_eq!(name, value);
            }
        }


        #[test]
        fn test_invalid_repology(value in r"[\s:]*") {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_bash::language()).unwrap();
            let tree = parser.parse(b"test", None).unwrap();
            let parent = tree.root_node();

            assert!(RepologyFilter::new(&format!("name:{value}"), &parent, &parent).is_err());
            assert!(RepologyFilter::new(&format!("name: {value}"), &parent, &parent).is_err());
        }

    }
}

//     //     #[rstest]
//     //     #[case("-package12@._+-")]
//     //     #[case(".package12@._+-")]
//     //     #[case("Package12@._+-")]
//     //     #[should_panic]
//     //     fn invalid_pkgnames(#[case] test_case: &str) {
//     //         Pkgname::new(test_case).unwrap();
//     //     }

//     //     #[test]
//     //     fn valid_pkgname() {
//     //         Pkgname::new("package12@._+-").unwrap();
//     //     }

//     //     #[test]
//     //     #[allow(clippy::too_many_lines)]
//     //     fn test_parser() {
//     //         let source_code = r#"pkgname='potato' # can also be an array,
// probably shouldn't be though     // pkgver='1.0.0' # this is the variable
// pkgver, can also be a function that will return dynamic version     //
// epoch=' 0' # force package to be seen as newer no matter what     //
// pkgdesc='Pretty obvious'     // url='https://potato.com'
//     // license="GPL-3.0-or-later WITH Classpath-exception-2.0 OR MIT AND AAL"
//     // arch=('any' 'x86_64')
//     // maintainer=('Henryws <hwengerstickel@pm.me>' 'wizard-28
// <wiz28@pm.me>')     // repology=([project]="$pkgname")
//     // provides=('foo' 'bar')
//     // mascot="ferris"
//     // source=(
//     // 	"https://potato.com/$pkgver.tar.gz"
//     // 	"potato.tar.gz::https://potato.com/$pkgver.tar.gz" # with a forced download name
//     // 	"$pkgname::git+https://github.com/pacstall/pacstall" # git repo
//     // 	"$pkgname::https://github.com/pacstall/pacstall/releases/download/2.0.1/pacstall-2.0.1.deb::repology" # use changelog with repology
//     // 	"$pkgname::git+https://github.com/pacstall/pacstall#branch=master" # git repo with branch
//     // 	"$pkgname::git+file://home/henry/pacstall/pacstall" # local git repo
//     // 	"magnet://xt=urn:btih:c4769d7000244e4cae9c054a83e38f168bf4f69f&
// dn=archlinux-2022.09.03-x86_64.iso" # magnet link     // 	"ftp://ftp.gnu.org/gnu/$pkgname/$pkgname-$pkgver.tar.xz" # ftp
//     // 	"patch-me-harder.patch::https://potato.com/patch-me.patch"
//     // ) # also source_x86_64=(), source_i386=()

//     // noextract=(
//     // 	"$pkgver.tar.gz"
//     // )

//     // sha256sums=(
//     //     "any_sha256sum_1"
//     // 	'SKIP'
//     // 	'SKIP'
//     // )

//     // sha256sums_x86_64=(
//     //     "x86_64_sha256sum_1"
//     //     "SKIP"
//     //     "x86_64_sha256sum_2"
//     //     "SKIP"
//     // )

//     // sha256sums_aarch64=(
//     //     "aarch64_sha256sum_1"
//     // )

//     // sha348sums=(
//     //     "sha348sum_1"
//     //     "SKIP"
//     // )

//     // sha512sums=(
//     //     "sha512sum_1"
//     //     "SKIP"
//     // )

//     // b2sums=(
//     //     "b2sum_1"
//     //     "SKIP"
//     // )

//     // optdepends=(
//     // 	'same as pacstall: yes'
//     // ) # rince and repeat optdepends_$arch=()

//     // depends=(
//     // 	'hashbrowns>=1.8.0'
//     // 	'mashed-potatos<=1.9.0'
//     // 	'gravy=2.3.0'
//     // 	'applesauce>3.0.0'
//     // 	'chicken<2.0.0'
//     // 	'libappleslices.so'
//     // 	'libdeepfryer.so=3'
//     // )

//     // makedepends=(
//     // 	'whisk'
//     // 	'onions'
//     // )

//     // checkdepends=(
//     // 	'customer_satisfaction'
//     // )

//     // ppa=('mcdonalds/ppa')

//     // provides=(
//     // 	'mashed-potatos'
//     // 	'aaaaaaaaaaaaaaaaaaaaaaaaaa'
//     // )

//     // conflicts=(
//     // 	'KFC'
//     // 	'potato_rights'
//     // ) # can also do conflicts_$arch=()

//     // replaces=(
//     // 	'kidney_beans'
//     // )

//     // backup=(
//     // 	'etc/potato/prepare.conf'
//     // )

//     // options=(
//     // 	'!strip'
//     // 	'!docs'
//     // 	'etc'
//     // )

//     // groups=('potato-clan')

//     // prepare() {
//     // 	cd "$pkgname-$pkgver"
//     // 	patch -p1 -i "$srcdir/patch-me-harder.patch"
//     // }

//     // func() {
//     //     true
//     // }

//     // build() {
//     // 	cd "$pkgname-$pkgver"
//     // 	./configure --prefix=/usr
//     // 	make
//     // }

//     // check() {
//     // 	cd "$pkgname-$pkgver"
//     // 	make -k check
//     // }

//     // package() {
//     // 	cd "$pkgname-$pkgver"
//     // 	make DESTDIR="$pkgdir/" install
//     // }

//     // pre_install() {
//     // 	echo "potato"
//     // }

//     // post_install() {
//     // 	echo "potato"
//     // }

//     // pre_upgrade() {
//     // 	echo "potato"
//     // }

//     // post_upgrade() {
//     // 	echo "potato"
//     // }

//     // pre_remove() {
//     // 	echo "potato"
//     // }

//     // post_remove() {
//     // 	echo "potato"
//     // }"#
//     //         .trim();

//     //         let pacbuild = PacBuild::from_source(source_code).unwrap();

//     //         assert_eq!(pacbuild.pkgname,
// vec![Pkgname::new("potato").unwrap()]);     //         assert_eq!(
//     //             pacbuild.pkgver,
//     //             PkgverType::Variable(Pkgver::new("1.0.0").unwrap())
//     //         );
//     //         assert_eq!(pacbuild.epoch, Some(0));
//     //         assert_eq!(pacbuild.pkgdesc, Some("Pretty obvious".into()));
//     //         assert_eq!(pacbuild.url, Some("https://potato.com".into()));
//     //         assert_eq!(
//     //             pacbuild.license,
//     //             Some(
//     //                 Expression::parse("GPL-3.0-or-later WITH
// Classpath-exception-2.0 OR MIT AND AAL")     //                     .unwrap()
//     //             )
//     //         );
//     //         assert_eq!(
//     //             pacbuild.custom_variables,
//     //             Some(HashMap::from([("mascot".into(), "ferris".into())]))
//     //         );

//     //         assert_eq!(pacbuild.arch, vec!["any", "x86_64"]);
//     //         assert_eq!(
//     //             pacbuild.maintainer,
//     //             Some(vec![
//     //                 Maintainer {
//     //                     name: "Henryws".into(),
//     //                     email: Some("hwengerstickel@pm.me".into())
//     //                 },
//     //                 Maintainer {
//     //                     name: "wizard-28".into(),
//     //                     email: Some("wiz28@pm.me".into())
//     //                 }
//     //             ])
//     //         );
//     //         assert_eq!(pacbuild.noextract,
// Some(vec!["1.0.0.tar.gz".into()]));     //         assert_eq!(
//     //             pacbuild.sha256sums,
//     //             Some(HashMap::from([
//     //                 (
//     //                     "any".into(),
//     //                     vec![Some("any_sha256sum_1".into()), None, None]
//     //                 ),
//     //                 (
//     //                     "x86_64".into(),
//     //                     vec![
//     //                         Some("x86_64_sha256sum_1".into()),
//     //                         None,
//     //                         Some("x86_64_sha256sum_2".into()),
//     //                         None
//     //                     ]
//     //                 ),
//     //                 ("aarch64".into(),
// vec![Some("aarch64_sha256sum_1".into())])     //             ]))
//     //         );
//     //         assert_eq!(
//     //             pacbuild.sha348sums,
//     //             Some(HashMap::from([(
//     //                 "any".into(),
//     //                 vec![Some("sha348sum_1".into()), None]
//     //             )]))
//     //         );
//     //         assert_eq!(
//     //             pacbuild.sha512sums,
//     //             Some(HashMap::from([(
//     //                 "any".into(),
//     //                 vec![Some("sha512sum_1".into()), None]
//     //             )]))
//     //         );
//     //         assert_eq!(
//     //             pacbuild.b2sums,
//     //             Some(HashMap::from([(
//     //                 "any".into(),
//     //                 vec![Some("b2sum_1".into()), None]
//     //             )]))
//     //         );
//     //         assert_eq!(
//     //             pacbuild.prepare,
//     //             Some(
//     //                 "prepare () \n{ \n    cd \"$pkgname-$pkgver\";\n
// patch -p1 -i \     //                  \"$srcdir/patch-me-harder.patch\"\n}"
//     //                     .into()
//     //             )
//     //         );
//     //         assert_eq!(
//     //             pacbuild.build,
//     //             Some(
//     //                 "build () \n{ \n    cd \"$pkgname-$pkgver\";\n
// ./configure --prefix=/usr;\n    \     //                  make\n}"
//     //                     .into()
//     //             )
//     //         );
//     //         assert_eq!(
//     //             pacbuild.check,
//     //             Some("check () \n{ \n    cd \"$pkgname-$pkgver\";\n
// make -k check\n}".into())     //         );
//     //         // pub package: Option<String>,
//     //         // pub pre_install: Option<String>,
//     //         // pub post_install: Option<String>,
//     //         // pub pre_upgrade: Option<String>,
//     //         // pub post_upgrade: Option<String>,
//     //         // pub pre_remove: Option<String>,
//     //         // pub post_remove: Option<String>,
//     //         // pub custom_functions: Option<HashMap<String, String>>,
//     //     }
// }
