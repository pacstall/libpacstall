use std::collections::HashMap;
use std::process::Command;

use error_stack::{bail, ensure, report, IntoReport, Result, ResultExt};
use spdx::Expression;
use tree_sitter::{Parser, Query, QueryCursor};

use super::errors::{InvalidField, ParserError};
use crate::parser::errors::MissingField;

#[derive(Debug, PartialEq, Eq)]
pub struct Pkgname(String);

impl Pkgname {
    pub fn new(name: &str) -> Result<Self, InvalidField> {
        for (index, character) in name.chars().enumerate() {
            if index == 0 {
                ensure!(
                    character != '-',
                    report!(InvalidField).attach_printable(format!(
                        r#"`pkgname` ({name}) cannot start with a hyphen"#
                    ))
                );

                ensure!(
                    character != '.',
                    report!(InvalidField).attach_printable(format!(
                        r#"`pkgname` ({name}) cannot start with a period"#
                    ))
                );
            }

            ensure!(
                character.is_alphabetic() && character.is_lowercase()
                    || character.is_numeric()
                    || character == '@'
                    || character == '.'
                    || character == '_'
                    || character == '+'
                    || character == '-',
                report!(InvalidField).attach_printable(format!(
                    r#"`pkgname` ({name}) can only contain lowercase alphanumerics or @._+-"#
                ))
            );
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
    pub fn new(version: &str) -> Result<Self, InvalidField> {
        ensure!(
            version.chars().all(|character| {
                character.is_alphanumeric() || character == '.' || character == '_'
            }),
            report!(InvalidField).attach_printable(format!(
                r#"`pkgver` ({version}) can only contain letters, numbers, periods, and underscores"#
            ))
        );

        Ok(Self(version.into()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Maintainer {
    name: String,
    email: Option<String>,
}

impl Maintainer {
    pub fn new(maintainer: &str) -> Result<Self, InvalidField> {
        let mut split: Vec<String> = maintainer.split(" <").map(ToString::to_string).collect();

        ensure!(
            split.len() <= 2,
            report!(InvalidField).attach_printable(format!(
                "`maintainer` ({maintainer}) can only contain a name and an email address"
            ))
        );

        Ok(Self {
            name: match split.first() {
                Some(name) => name.into(),
                None => {
                    bail!(report!(InvalidField)
                        .attach_printable(format!("`maintainer` ({maintainer}) is missing a name")))
                },
            },
            email: match split.last_mut() {
                Some(email) => {
                    email.pop(); // Removes the trailing `>`
                    Some((*email).to_string())
                },
                None => None,
            },
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
    pub fn from_source(source_code: &str) -> Result<Self, ParserError> {
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

        ensure!(
            sourced_code.status.success(),
            report!(ParserError).attach_printable(String::from_utf8(sourced_code.stderr).unwrap())
        );

        let mut parser = Parser::new();

        parser.set_language(tree_sitter_bash::language()).unwrap();

        let tree = match parser.parse(sourced_code.stdout.clone(), None) {
            Some(tree) => tree,
            None => bail!(ParserError),
        };

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
                            let name = query_match.captures[0]
                                .node
                                .utf8_text(&sourced_code.stdout)
                                .unwrap();

                            let index = query_match.captures[1].index;

                            match index {
                                // It's a normal variable.
                                2 => {
                                    let value = Self::cleanup_rawstring(
                                        query_match.captures[1]
                                            .node
                                            .utf8_text(&sourced_code.stdout)
                                            .unwrap(),
                                    );

                                    match name {
                                        "pkgname" => {
                                            pkgname =
                                                Some(vec![Pkgname::new(value)
                                                    .change_context(ParserError)?]);
                                        },
                                        "pkgver" => {
                                            pkgver = Some(PkgverType::Variable(
                                                Pkgver::new(value).change_context(ParserError)?,
                                            ));
                                        },
                                        "epoch" => {
                                            epoch = Some(
                                                value
                                                    .parse()
                                                    .into_report()
                                                    .change_context(InvalidField)
                                                    .attach_printable_lazy(|| {
                                                        format!(
                                                            "`epoch` ({value} can only be a \
                                                             non-negative integer"
                                                        )
                                                    })
                                                    .change_context(ParserError)?,
                                            );
                                        },
                                        "pkgdesc" => {
                                            pkgdesc = Some(value.into());
                                        },
                                        "license" => {
                                            license = Some(
                                                Expression::parse(value)
                                                    .into_report()
                                                    .change_context(InvalidField)
                                                    .attach_printable("`license is invalid`")
                                                    .change_context(ParserError)?,
                                            );
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
                                    let value = Self::cleanup_rawstring(
                                        query_match.captures[1]
                                            .node
                                            .utf8_text(&sourced_code.stdout)
                                            .unwrap(),
                                    );

                                    match name {
                                        "arch" => match &mut arch {
                                            Some(arch) => arch.push(value.into()),
                                            None => arch = Some(vec![value.into()]),
                                        },
                                        "maintainer" => match &mut maintainer {
                                            Some(maintainer) => maintainer.push(
                                                Maintainer::new(value)
                                                    .change_context(ParserError)?,
                                            ),
                                            None => {
                                                maintainer = Some(vec![Maintainer::new(value)
                                                    .change_context(ParserError)?]);
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

        let pkgname = match pkgname {
            Some(pkgname) => pkgname,
            None => bail!(report!(MissingField)
                .attach_printable("`pkgname` is missing")
                .change_context(ParserError)),
        };

        let pkgver = match pkgver {
            Some(pkgver) => pkgver,
            None => bail!(report!(MissingField)
                .attach_printable("`pkgver` is missing")
                .change_context(ParserError)),
        };

        let arch = match arch {
            Some(arch) => arch,
            None => bail!(report!(MissingField)
                .attach_printable("`arch` is missing")
                .change_context(ParserError)),
        };

        // TODO: Possibly check if checksum and sources lengths match

        let pkgbuild = Self {
            pkgname,
            pkgver,
            epoch,
            pkgdesc,
            url,
            custom_variables,
            license,
            arch,
            maintainer,
            noextract,
            sha256sums,
            sha348sums,
            sha512sums,
            b2sums,
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
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("-package12@._+-")]
    #[case(".package12@._+-")]
    #[case("Package12@._+-")]
    #[should_panic]
    fn invalid_pkgnames(#[case] test_case: &str) { Pkgname::new(test_case).unwrap(); }

    #[test]
    fn valid_pkgname() { Pkgname::new("package12@._+-").unwrap(); }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_parser() {
        let source_code = r#"pkgname='potato' # can also be an array, probably shouldn't be though
pkgver='1.0.0' # this is the variable pkgver, can also be a function that will return dynamic version
epoch='0' # force package to be seen as newer no matter what
pkgdesc='Pretty obvious'
url='https://potato.com'
license="GPL-3.0-or-later WITH Classpath-exception-2.0 OR MIT AND AAL"
arch=('any' 'x86_64')
maintainer=('Henryws <hwengerstickel@pm.me>' 'wizard-28 <wiz28@pm.me>')
repology=([project]="$pkgname")
provides=('foo' 'bar')
mascot="ferris"
source=(
	"https://potato.com/$pkgver.tar.gz"
	"potato.tar.gz::https://potato.com/$pkgver.tar.gz" # with a forced download name
	"$pkgname::git+https://github.com/pacstall/pacstall" # git repo
	"$pkgname::https://github.com/pacstall/pacstall/releases/download/2.0.1/pacstall-2.0.1.deb::repology" # use changelog with repology
	"$pkgname::git+https://github.com/pacstall/pacstall#branch=master" # git repo with branch
	"$pkgname::git+file://home/henry/pacstall/pacstall" # local git repo
	"magnet://xt=urn:btih:c4769d7000244e4cae9c054a83e38f168bf4f69f&dn=archlinux-2022.09.03-x86_64.iso" # magnet link
	"ftp://ftp.gnu.org/gnu/$pkgname/$pkgname-$pkgver.tar.xz" # ftp
	"patch-me-harder.patch::https://potato.com/patch-me.patch"
) # also source_x86_64=(), source_i386=()

noextract=(
	"$pkgver.tar.gz"
)

sha256sums=(
    "any_sha256sum_1"
	'SKIP'
	'SKIP'
)

sha256sums_x86_64=(
    "x86_64_sha256sum_1"
    "SKIP"
    "x86_64_sha256sum_2"
    "SKIP"
)

sha256sums_aarch64=(
    "aarch64_sha256sum_1"
)

sha348sums=(
    "sha348sum_1"
    "SKIP"
)

sha512sums=(
    "sha512sum_1"
    "SKIP"
)

b2sums=(
    "b2sum_1"
    "SKIP"
)


optdepends=(
	'same as pacstall: yes'
) # rince and repeat optdepends_$arch=()

depends=(
	'hashbrowns>=1.8.0'
	'mashed-potatos<=1.9.0'
	'gravy=2.3.0'
	'applesauce>3.0.0'
	'chicken<2.0.0'
	'libappleslices.so'
	'libdeepfryer.so=3'
)

makedepends=(
	'whisk'
	'onions'
)

checkdepends=(
	'customer_satisfaction'
)

ppa=('mcdonalds/ppa')

provides=(
	'mashed-potatos'
	'aaaaaaaaaaaaaaaaaaaaaaaaaa'
)

conflicts=(
	'KFC'
	'potato_rights'
) # can also do conflicts_$arch=()

replaces=(
	'kidney_beans'
)

backup=(
	'etc/potato/prepare.conf'
)

options=(
	'!strip'
	'!docs'
	'etc'
)

groups=('potato-clan')

prepare() {
	cd "$pkgname-$pkgver"
	patch -p1 -i "$srcdir/patch-me-harder.patch"
}

func() {
    true
}

build() {
	cd "$pkgname-$pkgver"
	./configure --prefix=/usr
	make
}

check() {
	cd "$pkgname-$pkgver"
	make -k check
}

package() {
	cd "$pkgname-$pkgver"
	make DESTDIR="$pkgdir/" install
}

pre_install() {
	echo "potato"
}

post_install() {
	echo "potato"
}

pre_upgrade() {
	echo "potato"
}

post_upgrade() {
	echo "potato"
}

pre_remove() {
	echo "potato"
}

post_remove() {
	echo "potato"
}"#
        .trim();

        let pacbuild = PacBuild::from_source(source_code).unwrap();

        assert_eq!(pacbuild.pkgname, vec![Pkgname::new("potato").unwrap()]);
        assert_eq!(
            pacbuild.pkgver,
            PkgverType::Variable(Pkgver::new("1.0.0").unwrap())
        );
        assert_eq!(pacbuild.epoch, Some(0));
        assert_eq!(pacbuild.pkgdesc, Some("Pretty obvious".into()));
        assert_eq!(pacbuild.url, Some("https://potato.com".into()));
        assert_eq!(
            pacbuild.license,
            Some(
                Expression::parse("GPL-3.0-or-later WITH Classpath-exception-2.0 OR MIT AND AAL")
                    .unwrap()
            )
        );
        assert_eq!(
            pacbuild.custom_variables,
            Some(HashMap::from([("mascot".into(), "ferris".into())]))
        );

        assert_eq!(pacbuild.arch, vec!["any", "x86_64"]);
        assert_eq!(
            pacbuild.maintainer,
            Some(vec![
                Maintainer {
                    name: "Henryws".into(),
                    email: Some("hwengerstickel@pm.me".into())
                },
                Maintainer {
                    name: "wizard-28".into(),
                    email: Some("wiz28@pm.me".into())
                }
            ])
        );
        assert_eq!(pacbuild.noextract, Some(vec!["1.0.0.tar.gz".into()]));
        assert_eq!(
            pacbuild.sha256sums,
            Some(HashMap::from([
                (
                    "any".into(),
                    vec![Some("any_sha256sum_1".into()), None, None]
                ),
                (
                    "x86_64".into(),
                    vec![
                        Some("x86_64_sha256sum_1".into()),
                        None,
                        Some("x86_64_sha256sum_2".into()),
                        None
                    ]
                ),
                ("aarch64".into(), vec![Some("aarch64_sha256sum_1".into())])
            ]))
        );
        assert_eq!(
            pacbuild.sha348sums,
            Some(HashMap::from([(
                "any".into(),
                vec![Some("sha348sum_1".into()), None]
            )]))
        );
        assert_eq!(
            pacbuild.sha512sums,
            Some(HashMap::from([(
                "any".into(),
                vec![Some("sha512sum_1".into()), None]
            )]))
        );
        assert_eq!(
            pacbuild.b2sums,
            Some(HashMap::from([(
                "any".into(),
                vec![Some("b2sum_1".into()), None]
            )]))
        );
        assert_eq!(
            pacbuild.prepare,
            Some(
                "prepare () \n{ \n    cd \"$pkgname-$pkgver\";\n    patch -p1 -i \
                 \"$srcdir/patch-me-harder.patch\"\n}"
                    .into()
            )
        );
        assert_eq!(
            pacbuild.build,
            Some(
                "build () \n{ \n    cd \"$pkgname-$pkgver\";\n    ./configure --prefix=/usr;\n    \
                 make\n}"
                    .into()
            )
        );
        assert_eq!(
            pacbuild.check,
            Some("check () \n{ \n    cd \"$pkgname-$pkgver\";\n    make -k check\n}".into())
        );
        // pub package: Option<String>,
        // pub pre_install: Option<String>,
        // pub post_install: Option<String>,
        // pub pre_upgrade: Option<String>,
        // pub post_upgrade: Option<String>,
        // pub pre_remove: Option<String>,
        // pub post_remove: Option<String>,
        // pub custom_functions: Option<HashMap<String, String>>,
    }
}
