# ml-lattice-rs

Wrappers idiomáticos en Rust sobre los crates oficiales de RustCrypto para los
algoritmos de criptografía post-cuántica estandarizados por NIST en 2024.

| Crate | Estándar | Algoritmo base |
|---|---|---|
| `kyber` | NIST FIPS 203 | ML-KEM (CRYSTALS-Kyber) |
| `dilithium` | NIST FIPS 204 | ML-DSA (CRYSTALS-Dilithium) |

---

## Por qué existe este workspace

Los crates upstream (`ml-kem`, `ml-dsa`) son correctos pero exponen una API de
bajo nivel con tipos genéricos complejos. Este workspace los envuelve para:

- Ofrecer funciones de alto nivel (`keygen`, `sign`, `verify`, `encapsulate`,
  `decapsulate`) sin tener que importar traits de `ml-kem` / `ml-dsa`.
- Re-exportar los tipos que el resto del proyecto (`pq-reth`, `pq-wallet`)
  necesita en un único punto de importación.
- Aislar la dependencia de los crates upstream: si NIST actualiza el estándar
  solo hay que cambiar este workspace, no todo el proyecto.

---

## Estructura

```
ml-lattice-rs/
├── Cargo.toml          ← workspace raíz
├── kyber/
│   └── src/lib.rs      ← wrapper ML-KEM (Kyber)
└── dilithium/
    └── src/lib.rs      ← wrapper ML-DSA (Dilithium)
```

---

## kyber — ML-KEM (Key Encapsulation)

ML-KEM es un mecanismo de **encapsulación de clave** (KEM). Se usa para
establecer un secreto compartido entre dos partes sin que un ordenador cuántico
pueda recuperarlo (resistente a Shor).

### Niveles de seguridad

| Variante | Categoría NIST | Seguridad clásica equivalente |
|---|---|---|
| `kyber512` | 1 | AES-128 |
| `kyber768` | 3 | AES-192 (**recomendado**) |
| `kyber1024` | 5 | AES-256 |

### Uso rápido

```rust
use kyber::kyber768;

// Generación de claves
let (dk, ek) = kyber768::keygen();

// El emisor encapsula un secreto con la clave pública de encapsulación
let (ciphertext, shared_key_sender) = kyber768::encapsulate(&ek);

// El receptor recupera el mismo secreto con su clave privada
let shared_key_receiver = kyber768::decapsulate(&dk, &ciphertext);

assert_eq!(shared_key_sender.as_slice(), shared_key_receiver.as_slice());
```

### Tamaños de clave y ciphertext (kyber768)

| Elemento | Tamaño |
|---|---|
| Clave de encapsulación (pública) | 1184 bytes |
| Clave de decapsulación (privada) | 2400 bytes |
| Ciphertext | 1088 bytes |
| Secreto compartido | 32 bytes |

### Ejecutar tests

```bash
cd ml-lattice-rs
cargo test -p kyber
# 11/11 tests pass
```

---

## dilithium — ML-DSA (Firma Digital)

ML-DSA es un esquema de **firma digital** de clave pública. Se usa para
autenticar transacciones. A diferencia de ECDSA, las firmas ML-DSA **no son
recuperables**: la clave pública debe enviarse junto con la firma para
verificar.

### Niveles de seguridad

| Variante | Categoría NIST | Clave signing | Clave verifying | Firma |
|---|---|---|---|---|
| `dilithium44` | 2 | 2528 B | 1312 B | 2420 B |
| `dilithium65` | 3 | 4032 B | **1952 B** | **3309 B** (**recomendado**) |
| `dilithium87` | 5 | 4896 B | 2592 B | 4627 B |

El proyecto usa **ML-DSA-65** en todos los componentes que requieren firmas.

### Uso rápido

```rust
use dilithium::dilithium65;
use dilithium::signature::Keypair;

// Generación de claves
let sk = dilithium65::keygen();

// Firma
let msg = b"post-quantum blockchain transaction";
let sig = dilithium65::sign(&sk, msg);

// Verificación con la clave pública
let ok = dilithium65::verify(&sk.verifying_key(), msg, &sig);
assert!(ok.is_ok());
```

### Derivación de address Ethereum desde clave ML-DSA-65

La clave pública se convierte en dirección Ethereum con el mismo algoritmo que
ECDSA, aplicado sobre los bytes de la clave `VerifyingKey`:

```
address = keccak256(pk_bytes)[12..]   // últimos 20 bytes
```

### Re-exports disponibles

```rust
pub use ml_dsa::{
    EncodedSignature, EncodedVerifyingKey, KeyGen,
    MlDsa44, MlDsa65, MlDsa87,
    Signature, SigningKey, VerifyingKey,
};
pub use ml_dsa::signature;  // traits Keypair, Signer, Verifier
```

### Ejecutar tests

```bash
cd ml-lattice-rs
cargo test -p dilithium
# 11/11 tests pass
```

---

## Dependencias upstream

```toml
ml-kem  = "0.3.0-rc.2"   # FIPS 203
ml-dsa  = "0.1.0-rc.8"   # FIPS 204
```

Ambos son crates de [RustCrypto](https://github.com/RustCrypto) bajo revisión
activa para convertirse en versiones estables una vez el estándar NIST esté
publicado en su forma definitiva.

---

## Notas de seguridad

- Este código es **experimental**. No usar en producción con fondos reales.
- Los crates `ml-kem` y `ml-dsa` usan implementaciones de referencia; no están
  optimizados con instrucciones AVX2/NEON todavía.
- La generación de claves usa `getrandom` con el CSPRNG del sistema operativo.
