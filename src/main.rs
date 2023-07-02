use cpal::platform::Device;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputCallbackInfo, OutputCallbackInfo, StreamError};
use ringbuf::HeapRb;
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

	// cpal::default_host()
	// 	.output_devices()
	// 	.into_iter()
	// 	.flatten()
	// 	.for_each(|device| {
	// 		if let Ok(name) = device.name() {
	// 			println!("{:?}", name);
	// 		}
	// 		device
	// 			.supported_output_configs()
	// 			.into_iter()
	// 			.flatten()
	// 			.for_each(|config| {
	// 				println!("    {:?}", config);
	// 			})
	// 	});

	println!("Using input device: {:?}", m8_input.name()?);
	println!("Using output device: {:?}", default_output.name()?);

	let input_config: cpal::StreamConfig =
		m8_input.default_input_config()?.into();
	let latency_frames = (150.0 / 1_000.0) * input_config.sample_rate.0 as f32;
	let latency_samples =
		latency_frames as usize * input_config.channels as usize;

	let ring = HeapRb::<f32>::new(latency_samples * 2);
	let (mut producer, mut consumer) = ring.split();

	for _ in 0..latency_samples {
		producer
			.push(0.0)
			.expect("buffers should have twice as much space as needed");
	}

	let input_data_fn = move |data: &[f32], _: &InputCallbackInfo| {
		if data
			.into_iter()
			.map(|&sample| producer.push(sample))
			.any(|res| res.is_err())
		{
			eprintln!("output stream feel behind. need to increase latency");
		}
	};

	let output_data_fn = move |data: &mut [f32], _: &OutputCallbackInfo| {
		let mut fell_behind = false;
		for sample in data {
			*sample = match consumer.pop() {
				Some(s) => s,
				None => {
					fell_behind = true;
					0.0
				}
			}
		}
		if fell_behind {
			eprintln!("input stream feel behind. need to increase latency");
		}
	};

	// Build streams.
	println!(
		"Attempting to build both streams with f32 samples and `{:?}`.",
		input_config
	);

	let input_stream = m8_input.build_input_stream(
		&input_config,
		input_data_fn,
		err_fn,
		None,
	)?;

	let output_stream = default_output.build_output_stream(
		&input_config,
		output_data_fn,
		err_fn,
		None,
	)?;

	input_stream.play()?;
	output_stream.play()?;

	// Run for 3 seconds
	println!("Playing for 3 seconds... ");
	std::thread::sleep(std::time::Duration::from_secs(3));
	drop(input_stream);
	drop(output_stream);
	println!("Done!");

	Ok(())
}

fn err_fn(err: StreamError) {
	eprintln!("an error occurred on stream: {}", err);
}
