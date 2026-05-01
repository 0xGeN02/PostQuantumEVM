// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title PQVerify — ML-DSA-65 signature verification library
/// @notice Provides a Solidity interface to the ML-DSA-65 verify precompile at 0x0100.
/// @dev This precompile is only available on PostQuantumEVM chains. Calling on
///      standard Ethereum will revert (no code at 0x0100).
///
/// Input layout (packed, no ABI encoding):
///   [0..32]       msg_hash   — 32-byte SHAKE-256 hash of the signed message
///   [32..3341]    signature  — 3309-byte ML-DSA-65 raw signature
///   [3341..5293]  public_key — 1952-byte ML-DSA-65 raw verifying key
///
/// Output: 32 bytes (uint256) — 1 = valid, 0 = invalid
///
/// Gas cost: 50,000 (static)
library PQVerify {
    /// @notice Address of the ML-DSA-65 verify precompile.
    address internal constant PRECOMPILE = address(0x0100);

    /// @notice Expected signature length for ML-DSA-65.
    uint256 internal constant SIG_LEN = 3309;

    /// @notice Expected public key length for ML-DSA-65.
    uint256 internal constant PK_LEN = 1952;

    /// @notice Verify an ML-DSA-65 signature.
    /// @param msgHash  The 32-byte SHAKE-256 hash that was signed.
    /// @param signature The 3309-byte ML-DSA-65 signature.
    /// @param publicKey The 1952-byte ML-DSA-65 verifying key.
    /// @return valid True if the signature is valid for the given hash and key.
    function verify(
        bytes32 msgHash,
        bytes memory signature,
        bytes memory publicKey
    ) internal view returns (bool valid) {
        require(signature.length == SIG_LEN, "PQVerify: invalid sig length");
        require(publicKey.length == PK_LEN, "PQVerify: invalid pk length");

        bytes memory input = abi.encodePacked(msgHash, signature, publicKey);

        (bool success, bytes memory result) = PRECOMPILE.staticcall(input);

        if (!success || result.length != 32) {
            return false;
        }

        uint256 r;
        assembly {
            r := mload(add(result, 32))
        }
        return r == 1;
    }

    /// @notice Verify and revert if invalid (convenience wrapper).
    /// @param msgHash  The 32-byte SHAKE-256 hash that was signed.
    /// @param signature The 3309-byte ML-DSA-65 signature.
    /// @param publicKey The 1952-byte ML-DSA-65 verifying key.
    function verifyOrRevert(
        bytes32 msgHash,
        bytes memory signature,
        bytes memory publicKey
    ) internal view {
        require(verify(msgHash, signature, publicKey), "PQVerify: invalid signature");
    }
}
