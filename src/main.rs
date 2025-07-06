#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::collapsible_if)]
#![deny(clippy::needless_else)]
#![deny(clippy::branches_sharing_code)]
#![deny(clippy::if_same_then_else)]

fn main() {
    sutra::cli::run();
}
