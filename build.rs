use std::io;



fn main() -> io::Result<()> {
	#[allow(unused_variables)]
	let status: io::Result<()> = Ok(());

	#[cfg(target_family = "windows")]
	let status = on_windows::main();

	status
}



#[cfg(target_family = "windows")]
mod on_windows {
	use std::io;
	use std::env;
	use winres::WindowsResource;

	pub fn main() -> io::Result<()> {
		static_vcruntime::metabuild();
		if env::var_os("CARGO_CFG_WINDOWS").is_some() {
			WindowsResource::new()
				.set_icon("assets/icon.ico")
				.compile()?;
		}
		Ok(())
	}
}
