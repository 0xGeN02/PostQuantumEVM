//! Transaction encoding benchmark: PQ (ML-DSA-65) vs Classical (ECDSA)
//!
//! Measures:
//! - Transaction serialization size
//! - RLP encoding/decoding time
//! - Overall transaction processing overhead

use alloy_primitives::U256;
use alloy_rlp::{Encodable, RlpEncodable};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ─── PQ Transaction (ML-DSA-65) ──────────────────────────────────────────────

#[derive(RlpEncodable)]
struct PqTxRlpFields {
    chain_id: u64,
    nonce: u64,
    gas_price: u128,
    gas_limit: u64,
    to: alloy_rlp::Bytes,
    value: U256,
    input: alloy_rlp::Bytes,
    signature: alloy_rlp::Bytes,  // 3309 bytes for ML-DSA-65
    public_key: alloy_rlp::Bytes, // 1952 bytes for ML-DSA-65
}

// ─── Classical Transaction (ECDSA) ───────────────────────────────────────────

#[derive(RlpEncodable)]
struct ClassicalTxRlpFields {
    nonce: u64,
    gas_price: u128,
    gas_limit: u64,
    to: alloy_rlp::Bytes,
    value: U256,
    input: alloy_rlp::Bytes,
    v: u64,
    r: U256,
    s: U256,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_pq_tx() -> PqTxRlpFields {
    PqTxRlpFields {
        chain_id: 20561,
        nonce: 42,
        gas_price: 1_000_000_000, // 1 gwei
        gas_limit: 21_000,
        to: alloy_rlp::Bytes::copy_from_slice(&[0xABu8; 20]),
        value: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
        input: alloy_rlp::Bytes::new(),
        signature: alloy_rlp::Bytes::copy_from_slice(&[0xAAu8; 3309]),
        public_key: alloy_rlp::Bytes::copy_from_slice(&[0xBBu8; 1952]),
    }
}

fn make_classical_tx() -> ClassicalTxRlpFields {
    ClassicalTxRlpFields {
        nonce: 42,
        gas_price: 1_000_000_000,
        gas_limit: 21_000,
        to: alloy_rlp::Bytes::copy_from_slice(&[0xABu8; 20]),
        value: U256::from(1_000_000_000_000_000_000u128),
        input: alloy_rlp::Bytes::new(),
        v: 28,
        r: U256::from(0x1234567890abcdef_u128),
        s: U256::from(0xfedcba0987654321_u128),
    }
}

// ─── Benchmarks ──────────────────────────────────────────────────────────────

fn bench_tx_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("tx_encoding");

    let pq_tx = make_pq_tx();
    let classical_tx = make_classical_tx();

    group.bench_function("PQ tx RLP encode", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(5400);
            out.push(0x50u8); // PQ_TX_TYPE
            pq_tx.encode(&mut out);
            black_box(out)
        });
    });

    group.bench_function("Classical tx RLP encode", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(200);
            classical_tx.encode(&mut out);
            black_box(out)
        });
    });

    group.finish();
}

fn bench_tx_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("tx_size");

    // Just measure the encoded sizes (not really a benchmark, but useful for reporting)
    let pq_tx = make_pq_tx();
    let classical_tx = make_classical_tx();

    let mut pq_buf = Vec::new();
    pq_buf.push(0x50u8);
    pq_tx.encode(&mut pq_buf);

    let mut classical_buf = Vec::new();
    classical_tx.encode(&mut classical_buf);

    // Print sizes (these will show up in criterion output)
    println!("\n=== Transaction Size Comparison ===");
    println!("PQ transaction (ML-DSA-65):   {} bytes", pq_buf.len());
    println!(
        "Classical transaction (ECDSA): {} bytes",
        classical_buf.len()
    );
    println!(
        "Size ratio (PQ/Classical):     {:.1}x",
        pq_buf.len() as f64 / classical_buf.len() as f64
    );
    println!(
        "Overhead:                      +{} bytes",
        pq_buf.len() - classical_buf.len()
    );
    println!("===================================\n");

    group.bench_function("PQ tx encode (5.3KB)", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(5400);
            out.push(0x50u8);
            pq_tx.encode(&mut out);
            black_box(out.len())
        });
    });

    group.bench_function("Classical tx encode (110B)", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(200);
            classical_tx.encode(&mut out);
            black_box(out.len())
        });
    });

    group.finish();
}

criterion_group!(benches, bench_tx_encoding, bench_tx_size);
criterion_main!(benches);
