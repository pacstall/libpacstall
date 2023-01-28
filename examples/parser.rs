use libpacstall::parser::pacbuild::PacBuild;
use miette::Result;

fn main() -> Result<()> {
    let k = 0;
    dbg!(PacBuild::from_source(
        r#"
pkgname='te'
pkgver="1.0"
epoch="21"
arch=("any")
#maintainer=("foo <" "bar <>" "biz <biz1@qux.com> <biz2@qux.com>")
license="MIT"
ppa=("lol/kol")
#depends=("foods" "bar:  h")
#optdepends=("foo: ldsfadvnvbnvbnvnvnnvbnfds")
repology=("project: foo" "visiblename: distrotube")
sources=("git+file:///home/wizard/tmp/::repology")
"#
        .trim(),
    )?);
    Ok(())
}
