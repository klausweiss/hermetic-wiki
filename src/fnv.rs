//! 64-bit FNV-1a: a stable hash (never randomly keyed) for versions, ids and seeds.

/// Starting state of the hash.
pub const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// The hash of `bytes`.
pub fn hash(bytes: &[u8]) -> u64 {
    extend(OFFSET, bytes)
}

/// Folds `bytes` into the hash state `h`, for hashing several pieces as one.
pub fn extend(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}
