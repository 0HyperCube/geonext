/// Number of frames of smoothing to be applied
const SMOOTHING: usize = 10;

#[derive(Default, Debug)]
/// Stores time and frame rate
pub struct Time {
	delta_times: [f32; SMOOTHING],
	pub total_frames: usize,
	pub smooth_delta: f32,
	pub delta_time: f32,
	pub time: f32,
}

impl Time {
	/// Construct a new, empty time
	#[inline]
	pub fn new() -> Self {
		Self::default()
	}

	/// Called once every frame, this updates the time as well as calculating frame time
	pub fn update_time(&mut self, time: f32) {
		self.delta_time = time - self.time;
		self.time = time;
		self.delta_times[self.total_frames % SMOOTHING] = self.delta_time;
		self.total_frames += 1;
		let data_points = self.total_frames.min(SMOOTHING);
		self.smooth_delta = self.delta_times[0..data_points].iter().sum::<f32>() / data_points as f32;
	}

	/// Time in seconds since application start
	pub fn seconds(&self) -> f32 {
		self.time / 1000.
	}

	/// The peak framerate in the last 10 frames
	pub fn peak_frametime(&self) -> f32 {
		self.delta_times.iter().max_by(|&x, &y| x.partial_cmp(y).unwrap()).map(|&x| x).unwrap_or_default()
	}
}
