// std
use std::error::Error;
// crates.io
use vergen_gitcl::{CargoBuilder, Emitter, GitclBuilder};

fn main() -> Result<(), Box<dyn Error>> {
	let mut emitter = Emitter::default();

	emitter.add_instructions(&CargoBuilder::default().target_triple(true).build()?)?;

	// Disable the git version if installed from <https://crates.io>.
	if emitter.add_instructions(&GitclBuilder::default().sha(true).build()?).is_err() {
		println!("cargo:rustc-env=VERGEN_GIT_SHA=crates.io");
	}

	emitter.emit()?;

	Ok(())
}
