use criterion::{black_box, criterion_group, criterion_main, Criterion};
use libpacstall::parser::pacbuild::PacBuild;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("parser", |b| b.iter(|| PacBuild::from_source(black_box(r#"pkgname='potato' # can also be an array, probably shouldn't be though
pkgver='1.0.0' # this is the variable pkgver, can also be a function that will return dynamic version
epoch='0' # force package to be seen as newer no matter what
pkgdesc='Pretty obvious'
url='https://potato.com'
license="Apache-2.0 OR MIT"
arch=('any' 'x86_64')
maintainer=('Henryws <hwengerstickel@pm.me>' 'Wizard-28 <wiz28@pm.me> <alternate_wiz28@pm.me>')
repology=("project: $pkgname")
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
	'e69fcf51c211772d4f193f3dc59b1e91607bea7e53999f1d5e03ba401e5da969'
	'SKIP'
	'SKIP'
	'etc'
) # can also do sha256sums_x86_64=(), repeat for sha384, sha512, and b2

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

incompatible=('debian::jessy' 'ubuntu::20.04')

prepare() {
	cd "$pkgname-$pkgver"
	patch -p1 -i "$srcdir/patch-me-harder.patch"
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
))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
