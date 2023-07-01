use cpal::platform::Device;
use cpal::traits::{DeviceTrait, HostTrait};
use std::error;

fn get_m8_input() -> Option<Device> {
	cpal::default_host()
		.devices()
		.into_iter()
		.flatten()
		.find(|device| {
			device.default_input_config().is_ok()
				&& device.name().is_ok_and(|name| name.contains("M8"))
		})
}

fn main() -> Result<(), Box<dyn error::Error>> {
	let m8_input = get_m8_input().expect("failed to find M8 input device");
	let default_output = cpal::default_host()
		.default_output_device()
		.expect("failed to find default output device");

	println!("Using input device: {:?}", m8_input.name()?);
	println!("Using output device: {:?}", default_output.name()?);

	Ok(())
}
