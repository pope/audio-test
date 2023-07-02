use cpal::platform::Device;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputCallbackInfo, OutputCallbackInfo, StreamError};
use ringbuf::HeapRb;

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

fn main() -> anyhow::Result<()> {
	let m8_input = get_m8_input().expect("failed to find M8 input device");
	let default_output = cpal::default_host()
		.default_output_device()
		.expect("failed to find default output device");

	println!("Using input device: {:?}", m8_input.name()?);
	println!("Using output device: {:?}", default_output.name()?);

	let input_config: cpal::StreamConfig =
		m8_input.default_input_config()?.into();

	let output_config_supported = default_output
		.supported_output_configs()
		.into_iter()
		.flatten()
		.any(|config| {
			config.channels() == input_config.channels
				&& config.min_sample_rate() >= input_config.sample_rate
				&& config.max_sample_rate() <= input_config.sample_rate
		});
	if !output_config_supported {
		let mut msg = String::from("Unable to find output config\n\n");
		msg.push_str("Found:\n");
		default_output
			.supported_output_configs()
			.into_iter()
			.flatten()
			.map(|device| format!("    {:?}\n", device))
			.for_each(|device| {
				msg.push_str(&device);
			});
		msg.push_str("\nExpected:\n");
		let input_config = format!("    {:?}\n", input_config);
		msg.push_str(&input_config);
		return Err(anyhow::Error::msg(msg));
	}

	let latency_frames = 0.150 * input_config.sample_rate.0 as f32;
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
			.iter()
			.map(|&sample| producer.push(sample))
			.any(|res| res.is_err())
		{
			eprintln!("output stream fell behind. need to increase latency");
		}
	};

	let output_data_fn = move |data: &mut [f32], _: &OutputCallbackInfo| {
		if data
			.iter_mut()
			.map(|sample| {
				let (val, success) = match consumer.pop() {
					Some(s) => (s, true),
					None => (0.0, false),
				};
				*sample = val;
				success
			})
			.any(|ok| !ok)
		{
			eprintln!("input stream fell behind. need to increase latency");
		}
	};

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

	input_stream
		.play()
		.expect("unable to start M8 input stream");
	output_stream.play().expect("unable to start output stream");

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
