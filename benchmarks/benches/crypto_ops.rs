//! Cryptographic operations benchmark: ML-DSA-65 vs ECDSA/secp256k1
//!
//! Measures:
//! - Key generation (keygen)
//! - Signature creation (sign)
//! - Signature verification (verify)
//! - SHAKE-256 vs Keccak-256 hashing at various input sizes

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rand::RngCore;

// ─── ML-DSA-65 benchmarks ────────────────────────────────────────────────────

mod mldsa {
    use dilithium::dilithium65;
    use dilithium::signature::Keypair;

    pub fn keygen() -> dilithium::SigningKey<dilithium::MlDsa65> {
        dilithium65::keygen()
    }

    pub fn sign(sk: &dilithium::SigningKey<dilithium::MlDsa65>, msg: &[u8]) -> dilithium::Signature<dilithium::MlDsa65> {
        dilithium65::sign(sk, msg)
    }

    pub fn verify(
        pk: &dilithium::VerifyingKey<dilithium::MlDsa65>,
        msg: &[u8],
        sig: &dilithium::Signature<dilithium::MlDsa65>,
    ) -> bool {
        dilithium65::verify(pk, msg, sig).is_ok()
    }

    pub fn get_vk(sk: &dilithium::SigningKey<dilithium::MlDsa65>) -> dilithium::VerifyingKey<dilithium::MlDsa65> {
        sk.verifying_key().clone()
    }

    pub fn encoded_vk_bytes(sk: &dilithium::SigningKey<dilithium::MlDsa65>) -> Vec<u8> {
        sk.verifying_key().encode().as_slice().to_vec()
    }
}

// ─── ECDSA/secp256k1 benchmarks ─────────────────────────────────────────────

mod ecdsa_secp256k1 {
    use secp256k1::{Message, Secp256k1, SecretKey};

    pub fn keygen() -> (SecretKey, secp256k1::PublicKey) {
        let secp = Secp256k1::new();
        let sk = SecretKey::new(&mut rand::thread_rng());
        let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
        (sk, pk)
    }

    pub fn sign(sk: &SecretKey, msg_hash: &[u8; 32]) -> secp256k1::ecdsa::RecoverableSignature {
        let secp = Secp256k1::new();
        let msg = Message::from_digest_slice(msg_hash).unwrap();
        secp.sign_ecdsa_recoverable(&msg, sk)
    }

    pub fn verify(pk: &secp256k1::PublicKey, msg_hash: &[u8; 32], sig: &secp256k1::ecdsa::Signature) -> bool {
        let secp = Secp256k1::new();
        let msg = Message::from_digest_slice(msg_hash).unwrap();
        secp.verify_ecdsa(&msg, sig, pk).is_ok()
    }

    pub fn recover(msg_hash: &[u8; 32], sig: &secp256k1::ecdsa::RecoverableSignature) -> secp256k1::PublicKey {
        let secp = Secp256k1::new();
        let msg = Message::from_digest_slice(msg_hash).unwrap();
        secp.recover_ecdsa(&msg, sig).unwrap()
    }
}

// ─── Hash function benchmarks ────────────────────────────────────────────────

mod hashes {
    use sha3::{Shake256, digest::{ExtendableOutput, Update, XofReader}};
    use tiny_keccak::{Hasher, Keccak};

    pub fn shake256(data: &[u8]) -> [u8; 32] {
        let mut h = Shake256::default();
        h.update(data);
        let mut out = [0u8; 32];
        h.finalize_xof().read(&mut out);
        out
    }

    pub fn keccak256(data: &[u8]) -> [u8; 32] {
        let mut h = Keccak::v256();
        h.update(data);
        let mut out = [0u8; 32];
        h.finalize(&mut out);
        out
    }
}

// ─── Benchmark groups ────────────────────────────────────────────────────────

fn bench_keygen(c: &mut Criterion) {
    let mut group = c.benchmark_group("keygen");

    group.bench_function("ML-DSA-65", |b| {
        b.iter(|| black_box(mldsa::keygen()));
    });

    group.bench_function("ECDSA/secp256k1", |b| {
        b.iter(|| black_box(ecdsa_secp256k1::keygen()));
    });

    group.finish();
}

fn bench_sign(c: &mut Criterion) {
    let mut group = c.benchmark_group("sign");

    // ML-DSA-65
    let mldsa_sk = mldsa::keygen();
    let msg = [0x42u8; 32];

    group.bench_function("ML-DSA-65", |b| {
        b.iter(|| black_box(mldsa::sign(&mldsa_sk, &msg)));
    });

    // ECDSA
    let (ecdsa_sk, _) = ecdsa_secp256k1::keygen();
    let msg_hash = [0x42u8; 32];

    group.bench_function("ECDSA/secp256k1", |b| {
        b.iter(|| black_box(ecdsa_secp256k1::sign(&ecdsa_sk, &msg_hash)));
    });

    group.finish();
}

fn bench_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify");

    // ML-DSA-65
    let mldsa_sk = mldsa::keygen();
    let msg = [0x42u8; 32];
    let mldsa_sig = mldsa::sign(&mldsa_sk, &msg);
    let mldsa_pk = mldsa::get_vk(&mldsa_sk);

    group.bench_function("ML-DSA-65 verify", |b| {
        b.iter(|| black_box(mldsa::verify(&mldsa_pk, &msg, &mldsa_sig)));
    });

    // ECDSA verify (standard)
    let (ecdsa_sk, ecdsa_pk) = ecdsa_secp256k1::keygen();
    let msg_hash = [0x42u8; 32];
    let ecdsa_sig_rec = ecdsa_secp256k1::sign(&ecdsa_sk, &msg_hash);
    let ecdsa_sig = ecdsa_sig_rec.to_standard();

    group.bench_function("ECDSA/secp256k1 verify", |b| {
        b.iter(|| black_box(ecdsa_secp256k1::verify(&ecdsa_pk, &msg_hash, &ecdsa_sig)));
    });

    // ecrecover (recovery from recoverable sig — this is what the precompile does)
    group.bench_function("ecrecover (secp256k1 recover)", |b| {
        b.iter(|| black_box(ecdsa_secp256k1::recover(&msg_hash, &ecdsa_sig_rec)));
    });

    group.finish();
}

fn bench_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash");

    // Test different input sizes
    let sizes: &[usize] = &[32, 64, 128, 256, 512, 1024, 4096, 8192];

    for &size in sizes {
        let mut data = vec![0u8; size];
        rand::thread_rng().fill_bytes(&mut data);

        group.bench_with_input(BenchmarkId::new("SHAKE-256", size), &data, |b, d| {
            b.iter(|| black_box(hashes::shake256(d)));
        });

        group.bench_with_input(BenchmarkId::new("Keccak-256", size), &data, |b, d| {
            b.iter(|| black_box(hashes::keccak256(d)));
        });
    }

    group.finish();
}

fn bench_address_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("address_derivation");

    // PQ: shake256(pk)[12..32]  (pk = 1952 bytes)
    let mldsa_sk = mldsa::keygen();
    let pk_bytes = mldsa::encoded_vk_bytes(&mldsa_sk);

    group.bench_function("PQ (SHAKE-256 of 1952B pk)", |b| {
        b.iter(|| {
            let hash = hashes::shake256(&pk_bytes);
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&hash[12..32]);
            black_box(addr)
        });
    });

    // Classical: keccak256(pk)[12..32]  (pk = 64 bytes uncompressed, minus prefix)
    let (_, ecdsa_pk) = ecdsa_secp256k1::keygen();
    let pk_uncompressed = ecdsa_pk.serialize_uncompressed();

    group.bench_function("Classical (Keccak-256 of 64B pk)", |b| {
        b.iter(|| {
            let hash = hashes::keccak256(&pk_uncompressed[1..]); // skip 0x04 prefix
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&hash[12..32]);
            black_box(addr)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_keygen,
    bench_sign,
    bench_verify,
    bench_hash,
    bench_address_derivation
);
criterion_main!(benches);
