use std::sync::atomic::{AtomicU64, Ordering};

static LOCAL_NOISE_CALL_COUNTER: AtomicU64 = AtomicU64::new(0);

struct XorShift64 {
	state: u64,
}
impl XorShift64 {
	fn new(seed: u64) -> Self {
		let state = if seed == 0 { 0x4D59_5DF4_D0F3_3173 } else { seed };

		Self { state }
	}

	fn next_u64(&mut self) -> u64 {
		let mut x = self.state;

		x ^= x << 13;
		x ^= x >> 7;
		x ^= x << 17;
		self.state = x;

		x
	}

	fn next_f32(&mut self) -> f32 {
		// Map to [0, 1). Keep 24 bits of precision for a stable f32.
		let bits = (self.next_u64() >> 40) as u32;

		(bits as f32) / ((1_u32 << 24) as f32)
	}
}

pub(super) fn seed_for_query_call(query: &str) -> u64 {
	let query_hash = blake3::hash(query.as_bytes());
	let mut seed_bytes = [0_u8; 8];

	seed_bytes.copy_from_slice(&query_hash.as_bytes()[..8]);

	let call_idx = LOCAL_NOISE_CALL_COUNTER.fetch_add(1, Ordering::Relaxed);
	let mut seed = u64::from_le_bytes(seed_bytes);

	seed ^= call_idx.wrapping_mul(0x9E37_79B9_7F4A_7C15);

	seed
}

pub(super) fn signed_unit_noise(seed: u64, index: usize) -> f32 {
	let mut rng = XorShift64::new(seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
	let u = rng.next_f32();

	(u * 2.0) - 1.0
}
