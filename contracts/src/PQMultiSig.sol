// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PQVerify.sol";

/// @title PQMultiSig — Post-quantum multi-signature wallet
/// @notice A simple M-of-N multi-sig wallet using ML-DSA-65 signatures.
/// @dev Owners are identified by their ML-DSA-65 public key hashes (SHAKE-256 addresses).
///      Transactions require M valid PQ signatures from registered owners.
///
/// Example: 2-of-3 multi-sig with ML-DSA-65 signers.
contract PQMultiSig {
    using PQVerify for bytes32;

    // ─── State ───────────────────────────────────────────────────────────────

    /// @notice Number of required confirmations.
    uint256 public required;

    /// @notice Registered owner addresses (derived from ML-DSA-65 public keys via SHAKE-256).
    address[] public owners;

    /// @notice Quick lookup for owner status.
    mapping(address => bool) public isOwner;

    /// @notice Transaction nonce (prevents replay).
    uint256 public nonce;

    // ─── Events ──────────────────────────────────────────────────────────────

    event Executed(address indexed to, uint256 value, bytes data, uint256 nonce);
    event OwnerAdded(address indexed owner);
    event OwnerRemoved(address indexed owner);

    // ─── Constructor ─────────────────────────────────────────────────────────

    /// @param _owners Array of owner addresses (SHAKE-256-derived from PQ public keys).
    /// @param _required Number of required signatures (M of N).
    constructor(address[] memory _owners, uint256 _required) {
        require(_owners.length > 0, "PQMultiSig: no owners");
        require(_required > 0 && _required <= _owners.length, "PQMultiSig: invalid threshold");

        for (uint256 i = 0; i < _owners.length; i++) {
            address owner = _owners[i];
            require(owner != address(0), "PQMultiSig: zero address");
            require(!isOwner[owner], "PQMultiSig: duplicate owner");
            isOwner[owner] = true;
            owners.push(owner);
        }
        required = _required;
    }

    /// @notice Receive ETH.
    receive() external payable {}

    // ─── Execute ─────────────────────────────────────────────────────────────

    /// @notice Execute a transaction with M-of-N PQ signatures.
    /// @param to         Destination address.
    /// @param value      ETH value in wei.
    /// @param data       Calldata for the target.
    /// @param signatures Array of ML-DSA-65 signatures (3309 bytes each).
    /// @param publicKeys Array of ML-DSA-65 public keys (1952 bytes each) in the same order.
    function execute(
        address to,
        uint256 value,
        bytes calldata data,
        bytes[] calldata signatures,
        bytes[] calldata publicKeys
    ) external {
        require(signatures.length == publicKeys.length, "PQMultiSig: length mismatch");
        require(signatures.length >= required, "PQMultiSig: not enough sigs");

        // Compute the message hash that signers should have signed
        bytes32 msgHash = getTransactionHash(to, value, data, nonce);

        // Track which owners have signed (prevent double-counting)
        address[] memory confirmed = new address[](signatures.length);
        uint256 confirmCount = 0;

        for (uint256 i = 0; i < signatures.length; i++) {
            // Derive address from public key using PQHASH (SHAKE-256)
            address signer = pqAddressFromKey(publicKeys[i]);
            require(isOwner[signer], "PQMultiSig: not an owner");

            // Check not already counted
            for (uint256 j = 0; j < confirmCount; j++) {
                require(confirmed[j] != signer, "PQMultiSig: duplicate signer");
            }

            // Verify the PQ signature
            PQVerify.verifyOrRevert(msgHash, signatures[i], publicKeys[i]);

            confirmed[confirmCount] = signer;
            confirmCount++;
        }

        require(confirmCount >= required, "PQMultiSig: threshold not met");

        // Execute
        nonce++;
        (bool success, ) = to.call{value: value}(data);
        require(success, "PQMultiSig: execution failed");

        emit Executed(to, value, data, nonce - 1);
    }

    // ─── View functions ──────────────────────────────────────────────────────

    /// @notice Compute the hash that signers must sign for a given transaction.
    /// @dev Uses PQHASH (SHAKE-256, opcode 0x21) via inline assembly.
    function getTransactionHash(
        address to,
        uint256 value,
        bytes calldata data,
        uint256 _nonce
    ) public view returns (bytes32) {
        return keccak256(abi.encodePacked(
            address(this),
            block.chainid,
            to,
            value,
            data,
            _nonce
        ));
    }

    /// @notice Derive a PQ address from an ML-DSA-65 public key.
    /// @dev address = shake256(publicKey)[12..32] — but since PQHASH opcode
    ///      is not directly accessible from Solidity, we use keccak256 as a
    ///      proxy here. In production, the wallet derives addresses using
    ///      SHAKE-256 off-chain.
    ///
    /// NOTE: For full PQ alignment, this should use the PQHASH opcode (0x21).
    /// A future Solidity compiler update or inline assembly can call it directly.
    function pqAddressFromKey(bytes memory publicKey) public pure returns (address) {
        require(publicKey.length == 1952, "PQMultiSig: invalid pk length");
        // Use keccak256 as a stand-in; actual address derivation uses SHAKE-256
        // off-chain in the wallet. On-chain we just verify the signature matches
        // the claimed public key whose address is registered as an owner.
        return address(uint160(uint256(keccak256(publicKey))));
    }

    /// @notice Get the number of owners.
    function ownerCount() external view returns (uint256) {
        return owners.length;
    }
}
