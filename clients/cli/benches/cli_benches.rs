/// Criterion benchmarks for CLI operations.
///
/// Run with: `cargo bench -p cli`
/// Results are written to `target/criterion/`.
use criterion::{black_box, criterion_group, criterion_main, Criterion};

// Re-use the library code directly — no subprocess needed.
// The CLI crate is a binary, so we reference the modules via `include!` paths
// or by compiling with `--lib`. Since it's a bin-only crate we inline the
// relevant logic here via the public functions exposed in each module.

// ── address derivation ───────────────────────────────────────────────────────

/// Benchmark deriving a Stellar public address from a secret key string.
fn bench_address_derivation(c: &mut Criterion) {
    // A well-known test secret key (not used on any real network).
    let secret = "SCZANGBA5RLMPI7JMTP2UX7BAYCHG4RKWU7RNKFBLTARBNZIVSWFSRA";

    c.bench_function("address_derivation", |b| {
        b.iter(|| {
            // Import inline to avoid needing a lib target.
            use stellar_strkey::ed25519::{PrivateKey as StrkeySecret, PublicKey as StrkeyPublic};
            use ed25519_dalek::SigningKey;

            let strkey = StrkeySecret::from_string(black_box(secret)).unwrap();
            let signing_key = SigningKey::from_bytes(&strkey.0);
            let vk = signing_key.verifying_key();
            let _addr = StrkeyPublic(vk.to_bytes()).to_string();
        })
    });
}

// ── XDR base64 encoding ──────────────────────────────────────────────────────

/// Benchmark base64 encoding of a typical XDR payload (~256 bytes).
fn bench_xdr_encode(c: &mut Criterion) {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let payload: Vec<u8> = (0u8..=255).collect(); // 256 bytes

    c.bench_function("xdr_encode_256b", |b| {
        b.iter(|| {
            let _encoded = STANDARD.encode(black_box(&payload));
        })
    });
}

/// Benchmark base64 decoding of a typical XDR payload (~256 bytes).
fn bench_xdr_decode(c: &mut Criterion) {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let payload: Vec<u8> = (0u8..=255).collect();
    let encoded = STANDARD.encode(&payload);

    c.bench_function("xdr_decode_256b", |b| {
        b.iter(|| {
            let _decoded = STANDARD.decode(black_box(encoded.as_bytes())).unwrap();
        })
    });
}

criterion_group!(benches, bench_address_derivation, bench_xdr_encode, bench_xdr_decode);
criterion_main!(benches);
