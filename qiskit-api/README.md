# qiskit-api — API de Simulación de Ataques Cuánticos

Servidor REST que simula ataques cuánticos contra las primitivas criptográficas
clásicas usadas en Ethereum. Construido con **FastAPI** y **Qiskit + Aer** para
demostrar por qué la blockchain necesita criptografía post-cuántica.

---

## Por qué existe esta API

Los ataques cuánticos a ECDSA (Shor) y a funciones hash (Grover) son teóricos
a escala real, pero **ya son demostrables en circuitos de juguete**. Esta API
los ejecuta en simuladores cuánticos para ilustrar:

1. **Algoritmo de Grover** — busca la preimagen de un hash en `O(√N)` pasos en
   vez de `O(N)`. En una función hash de 256 bits reduce la seguridad efectiva
   a 128 bits.
2. **Algoritmo de Shor** — resuelve el problema del logaritmo discreto en curvas
   elípticas (ECDLP) en tiempo polinomial. Con un ordenador cuántico
   suficientemente grande rompería secp256k1 y por tanto ECDSA.

---

## Estructura

```
qiskit-api/
├── src/
│   ├── main.py          ← servidor FastAPI, health check
│   ├── models.py        ← modelos Pydantic de request/response
│   └── routers/
│       ├── grover.py    ← endpoints /grover/*
│       └── shor.py      ← endpoints /shor/*
├── lib/                 ← implementaciones cuánticas
│   ├── grover.py        ← circuito Grover + oráculo Keccak toy
│   ├── keccak_toy.py    ← función hash de juguete (4 bits → 4 bits)
│   ├── shor.py          ← circuito Shor ECDLP
│   └── secp256k1_toy.py ← curva secp256k1 de juguete (p=17)
├── Dockerfile
├── docker-compose.yml   ← en la raíz del proyecto
└── requirements.txt
```

---

## Arranque

### Desarrollo local

```bash
cd qiskit-api
pip install -r requirements.txt
# o con uv:
uv sync

uvicorn src.main:app --reload --port 8888
```

### Docker

```bash
# Desde la raíz del proyecto PostQuantumEVM:
docker compose up qiskit-api

# La API queda disponible en http://localhost:8888
# Swagger UI:  http://localhost:8888/docs
# ReDoc:       http://localhost:8888/redoc
```

---

## Endpoints

### Health check

```
GET /
```

```json
{ "status": "ok", "service": "qiskit-api", "version": "0.2.0" }
```

---

### Grover — búsqueda de preimagen de hash

#### `GET /grover/lookup`

Devuelve la tabla completa de la función Keccak de juguete (4 bits → 4 bits,
16 entradas). Útil para elegir un `target_hash` válido antes de llamar al
siguiente endpoint.

```json
{
  "lookup": {
    "0000": "1010",
    "0001": "0110",
    "...": "..."
  }
}
```

#### `POST /grover/keccak`

Ejecuta el algoritmo de Grover para encontrar `x` tal que `keccak_toy(x) == target_hash`.

**Request:**
```json
{
  "target_hash": [1, 0, 1, 0]
}
```

**Response:**
```json
{
  "target_hash":    [1, 0, 1, 0],
  "preimage_found": [0, 0, 0, 0],
  "iterations":     1,
  "n_qubits":       4,
  "circuit_depth":  23,
  "success":        true
}
```

**Parámetros del circuito:**
- N = 16 estados (4 qubits)
- Iteraciones óptimas: `⌊π/4 · √(N/M)⌋` donde M es el número de soluciones
- El oráculo marca los estados que satisfacen `keccak_toy(x) = target`

---

### Shor — recuperación de clave privada ECDLP

#### `GET /shor/curve`

Devuelve los parámetros de la curva secp256k1 de juguete usada en la simulación:
`y² = x³ + 7 (mod 17)`.

```json
{
  "p": 17,
  "a": 0,
  "b": 7,
  "G": [15, 13],
  "N": 17,
  "order": 18,
  "subgroup_points": [[15,13], [2,10], "..."]
}
```

#### `POST /shor/ecdlp`

Recupera la clave privada `k` dada la clave pública `Q = k·G` usando el
algoritmo de Shor ECDLP en la curva de juguete.

**Request (con clave pública explícita):**
```json
{
  "public_key": [2, 10]
}
```

**Request (dejar que el servidor genere un keypair aleatorio):**
```json
{}
```

**Response:**
```json
{
  "public_key":        [2, 10],
  "private_key_found": 2,
  "private_key_real":  2,
  "success":           true,
  "n_qubits":          9,
  "circuit_depth":     47
}
```

---

## Advertencias sobre los simuladores de juguete

| Componente | Real (secp256k1) | Toy (esta API) |
|---|---|---|
| Tamaño del campo | 256 bits | 4 bits (Grover) / p=17 (Shor) |
| Qubits necesarios | ~2330 (Shor) | ~9 |
| Tiempo de cómputo | Décadas (hardware real actual) | Milisegundos (simulador clásico) |
| Objetivo | Demostración educativa | Igual |

Los ataques cuánticos **no son prácticos hoy** contra secp256k1 real (se
necesitarían miles de qubits tolerantes a fallos). Esta API sirve para entender
el mecanismo, no para atacar nada real.

---

## Relación con el resto del proyecto

```
PostQuantumEVM
├── qiskit-api      ← ESTA API: demuestra que ECDSA/Keccak son vulnerables
├── ml-lattice-rs   ← ML-KEM + ML-DSA: algoritmos resistentes al cuántico
├── pq-wallet       ← wallet que usa ML-DSA-65 en vez de ECDSA
└── pq-reth         ← cliente Ethereum modificado para aceptar txs ML-DSA-65
```

La API de Qiskit no forma parte del stack de producción. Es la **motivación**
del proyecto: mostrar por qué se necesitan ML-KEM y ML-DSA.
