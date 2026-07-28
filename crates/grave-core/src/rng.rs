use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use sha2::{Digest, Sha256};

pub fn render_rng(burial_id: &[u8; 32], domain: u8, values: &[u64]) -> ChaCha8Rng {
    let mut hasher = Sha256::new();
    hasher.update(burial_id);
    hasher.update([domain]);
    for value in values {
        hasher.update(value.to_le_bytes());
    }

    let digest = hasher.finalize();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&digest);
    ChaCha8Rng::from_seed(seed)
}
