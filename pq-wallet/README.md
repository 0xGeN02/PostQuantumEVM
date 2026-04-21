# pq-wallet — Wallet Post-Cuántica para Ethereum

Wallet de línea de comandos (CLI) para interactuar con la red **PostQuantumEVM**.
Genera claves ML-DSA-65 (CRYSTALS-Dilithium), las guarda cifradas en disco y
construye, firma y emite transacciones de tipo `0x04`.

Las claves **nunca salen del dispositivo** sin estar cifradas. No hay servidor,
no hay servicio en la nube.

---

## Estructura del proyecto

```
pq-wallet/
├── pq-wallet-core/         ← librería reutilizable
│   └── src/
│       ├── keygen.rs       ← generación de keypairs ML-DSA-65
│       ├── keystore.rs     ← cifrado/descifrado de keystore en disco
│       ├── signer.rs       ← firma de transacciones
│       ├── tx.rs           ← tipos PqTxRequest / PqSignedTx
│       ├── rpc.rs          ← cliente JSON-RPC mínimo
│       └── error.rs        ← WalletError
└── pq-wallet-cli/
    └── src/main.rs         ← binario `pq-wallet`
```

---

## Instalación

```bash
cd pq-wallet
cargo build --release -p pq-wallet-cli
# binario en: target/release/pq-wallet
```

---

## Comandos

### `new` — Generar keypair y guardar keystore

```bash
pq-wallet new --output mi-clave.json
# Pedirá passphrase interactivamente (no se muestra al escribir)

# O pasando la passphrase por flag (no recomendado en producción)
pq-wallet new --output mi-clave.json --passphrase "mi-passphrase-segura"
```

Salida:
```
Generating ML-DSA-65 keypair... done.
Keystore saved to:  mi-clave.json
Address:            0x4a8b2c1d...

WARNING: Anyone with your passphrase and keystore file can steal your funds.
         Keep them separate and make a backup.
```

---

### `address` — Ver dirección sin necesitar passphrase

```bash
pq-wallet address --keystore mi-clave.json
# 0x4a8b2c1d...
```

La clave pública está almacenada sin cifrar en el keystore (es pública), por lo
que la dirección se puede consultar sin introducir la passphrase.

---

### `balance` — Consultar saldo

```bash
pq-wallet balance \
    --keystore mi-clave.json \
    --rpc http://localhost:8545

# Address: 0x4a8b2c1d...
# Balance: 1.234567 ETH (1234567000000000000 wei)
```

---

### `send` — Construir, firmar y emitir transacción

```bash
pq-wallet send \
    --keystore mi-clave.json \
    --to 0xRecipientAddress \
    --value 1000000000000000000 \
    --gas-limit 21000 \
    --rpc http://localhost:8545
```

El `chain_id`, `nonce` y `gas_price` se obtienen automáticamente del nodo. Se
pueden sobreescribir:

```bash
pq-wallet send \
    --keystore mi-clave.json \
    --to 0xRecipientAddress \
    --value 0 \
    --gas-limit 100000 \
    --gas-price 1000000000 \
    --data 0xdeadbeef \
    --rpc http://localhost:8545
```

#### Modo dry-run (sin emitir)

```bash
pq-wallet send \
    --keystore mi-clave.json \
    --to 0xRecipientAddress \
    --gas-price 1000000000 \
    --dry-run

# Tx hash (local):  0xabc...
# Chain ID:         1
# Nonce:            0
# Gas price:        1000000000 wei
# Gas limit:        21000
# To:               0xRecipientAddress
# Value:            0 wei
# Raw tx size:      5342 bytes
# Raw tx hex:       0x04f9...
```

---

### `sign` — Firmar mensaje arbitrario

```bash
pq-wallet sign \
    --keystore mi-clave.json \
    "Hola mundo post-cuántico"

# Message:   Hola mundo post-cuántico
# Signer:    0x4a8b2c1d...
# Signature: 3a8f1c... (6618 hex chars = 3309 bytes)
```

---

## Formato del keystore

El keystore es un archivo JSON con el siguiente esquema:

```json
{
  "version": 1,
  "address": "0x4a8b2c1d...",
  "public_key": "<hex 3904 chars = 1952 bytes>",
  "crypto": {
    "kdf": "argon2id",
    "kdf_params": {
      "m_cost": 65536,
      "t_cost": 3,
      "p_cost": 4,
      "salt": "<hex 32 bytes>"
    },
    "cipher": "aes-256-gcm",
    "cipher_params": {
      "iv": "<hex 12 bytes>"
    },
    "ciphertext": "<hex 64 bytes>"
  }
}
```

### Lo que se cifra

Solo se cifran los **32 bytes de seed** del keypair ML-DSA-65 (no la clave
completa expandida de 4032 bytes). Al cargar, el seed se usa para regenerar
el keypair completo con `MlDsa65::from_seed(&seed)`.

### Seguridad del cifrado

| Capa | Algoritmo | Parámetros |
|---|---|---|
| Derivación de clave | Argon2id | m=64 MB, t=3 iteraciones, p=4 hilos |
| Cifrado | AES-256-GCM | nonce aleatorio de 12 bytes, tag de 16 bytes |

La dirección y la clave pública se guardan en claro (son datos públicos). Solo
el seed es secreto.

---

## pq-wallet-core — API de librería

Para integrar la funcionalidad en otro programa Rust:

```rust
use pq_wallet_core::{PqKeypair, Keystore, PqSigner};
use pq_wallet_core::tx::PqTxRequest;

// Generar keypair
let keypair = PqKeypair::generate();
println!("Address: {}", keypair.address());

// Guardar en keystore cifrado
keypair.save("/tmp/key.json", "mi-passphrase").unwrap();

// Cargar desde disco
let keypair = Keystore::load("/tmp/key.json", "mi-passphrase").unwrap();

// Firmar una transacción
let tx = PqTxRequest {
    chain_id: 1,
    nonce: 0,
    to: Some(recipient_address),
    value: 1_000_000_000_000_000_000u128,  // 1 ETH en wei
    gas_limit: 21_000,
    gas_price: 1_000_000_000,
    input: vec![],
};

let signer = PqSigner::new(&keypair);
let signed = signer.sign(tx);

// Encoding EIP-2718 listo para enviar por JSON-RPC
let raw = signed.encode();
```

### `RpcClient`

Cliente JSON-RPC mínimo (sin dependencias de alloy-providers):

```rust
use pq_wallet_core::RpcClient;

let client = RpcClient::new("http://localhost:8545");

let balance = client.get_balance(address).await?;
let nonce    = client.get_nonce(address).await?;
let chain_id = client.chain_id().await?;
let gas_price = client.gas_price().await?;
let tx_hash  = client.send_raw_transaction(&raw_hex).await?;
```

---

## Ejecutar tests

```bash
cd pq-wallet
cargo test
# 6/6 tests pass
```

---

## Notas de seguridad

- No usar en mainnet con fondos reales. Código experimental.
- La passphrase se lee por stdin sin echo. Evitar pasarla por `--passphrase`
  en producción (queda en el historial de shell).
- El archivo keystore **no es secreto por sí solo**: sin la passphrase no se
  puede recuperar la clave privada. Aun así, guárdalo en un lugar seguro.
- Hacer backup del keystore **y** de la passphrase por separado.
