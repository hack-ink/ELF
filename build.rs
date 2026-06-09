#![allow(missing_docs)]

use std::error::Error;

use vergen_gitcl::{Cargo, Emitter, Gitcl};

fn main() -> Result<(), Box<dyn Error>> {
	let mut emitter = Emitter::default();

	emitter.add_instructions(&Cargo::builder().target_triple(true).build())?;

	// Disable the git version if installed from <https://crates.io>.
	if emitter.add_instructions(&Gitcl::builder().sha(false).build()).is_err() {
		println!("cargo:rustc-env=VERGEN_GIT_SHA=crates.io");
	}

	emitter.emit()?;

	Ok(())
}
