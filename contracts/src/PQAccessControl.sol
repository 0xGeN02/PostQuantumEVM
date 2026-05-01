// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PQVerify.sol";

/// @title PQAccessControl — Post-quantum signature-gated access control
/// @notice Demonstrates using ML-DSA-65 signatures for on-chain authorization.
/// @dev Replaces the classical ECDSA-based ecrecover pattern with the PQ precompile.
///
/// Classical pattern (BROKEN on PQ chain):
///   bytes32 hash = keccak256(msg);
///   address signer = ecrecover(hash, v, r, s);  // REVERTS — precompile disabled
///
/// Post-quantum pattern:
///   bytes32 hash = shake256(msg);  // or keccak256 for compatibility
///   bool valid = PQVerify.verify(hash, signature, publicKey);
///   address signer = deriveAddress(publicKey);
contract PQAccessControl {
    using PQVerify for bytes32;

    // ─── State ───────────────────────────────────────────────────────────────

    /// @notice Contract owner (PQ-derived address).
    address public owner;

    /// @notice Mapping of role hashes to authorized PQ addresses.
    mapping(bytes32 => mapping(address => bool)) public hasRole;

    /// @notice Default admin role.
    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN");

    // ─── Events ──────────────────────────────────────────────────────────────

    event RoleGranted(bytes32 indexed role, address indexed account);
    event RoleRevoked(bytes32 indexed role, address indexed account);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    // ─── Constructor ─────────────────────────────────────────────────────────

    constructor() {
        owner = msg.sender;
        hasRole[ADMIN_ROLE][msg.sender] = true;
        emit RoleGranted(ADMIN_ROLE, msg.sender);
    }

    // ─── Modifiers ───────────────────────────────────────────────────────────

    modifier onlyOwner() {
        require(msg.sender == owner, "PQAccessControl: not owner");
        _;
    }

    modifier onlyRole(bytes32 role) {
        require(hasRole[role][msg.sender], "PQAccessControl: missing role");
        _;
    }

    // ─── Admin functions ─────────────────────────────────────────────────────

    /// @notice Grant a role to an address (admin-only).
    function grantRole(bytes32 role, address account) external onlyRole(ADMIN_ROLE) {
        hasRole[role][account] = true;
        emit RoleGranted(role, account);
    }

    /// @notice Revoke a role from an address (admin-only).
    function revokeRole(bytes32 role, address account) external onlyRole(ADMIN_ROLE) {
        hasRole[role][account] = false;
        emit RoleRevoked(role, account);
    }

    /// @notice Transfer ownership with PQ signature verification.
    /// @dev The new owner must prove they control the key by providing a signature
    ///      over a domain-separated message.
    /// @param newOwner     The new owner address.
    /// @param publicKey    ML-DSA-65 public key (1952 bytes) of the new owner.
    /// @param signature    ML-DSA-65 signature (3309 bytes) proving key ownership.
    function transferOwnershipPQ(
        address newOwner,
        bytes calldata publicKey,
        bytes calldata signature
    ) external onlyOwner {
        // Verify the new owner controls the claimed public key
        bytes32 msgHash = keccak256(abi.encodePacked(
            "PQAccessControl::transferOwnership",
            address(this),
            block.chainid,
            newOwner
        ));

        PQVerify.verifyOrRevert(msgHash, signature, publicKey);

        address previousOwner = owner;
        owner = newOwner;
        hasRole[ADMIN_ROLE][previousOwner] = false;
        hasRole[ADMIN_ROLE][newOwner] = true;

        emit OwnershipTransferred(previousOwner, newOwner);
        emit RoleGranted(ADMIN_ROLE, newOwner);
    }

    // ─── Signature-gated operations ──────────────────────────────────────────

    /// @notice Execute an arbitrary call, authorized by a PQ signature from the owner.
    /// @dev This allows meta-transactions: anyone can submit the tx, but only the
    ///      owner's PQ signature can authorize it.
    /// @param to         Target contract/address.
    /// @param value      ETH value to send.
    /// @param data       Calldata.
    /// @param deadline   Timestamp after which the signature expires.
    /// @param publicKey  Owner's ML-DSA-65 public key.
    /// @param signature  Owner's ML-DSA-65 signature over the call parameters.
    function executeWithSignature(
        address to,
        uint256 value,
        bytes calldata data,
        uint256 deadline,
        bytes calldata publicKey,
        bytes calldata signature
    ) external returns (bytes memory) {
        require(block.timestamp <= deadline, "PQAccessControl: expired");

        bytes32 msgHash = keccak256(abi.encodePacked(
            "PQAccessControl::execute",
            address(this),
            block.chainid,
            to,
            value,
            keccak256(data),
            deadline
        ));

        PQVerify.verifyOrRevert(msgHash, signature, publicKey);

        (bool success, bytes memory result) = to.call{value: value}(data);
        require(success, "PQAccessControl: call failed");
        return result;
    }
}
