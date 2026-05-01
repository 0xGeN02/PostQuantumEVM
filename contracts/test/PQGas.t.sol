// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/PQVerify.sol";
import "../src/PQHash.sol";

/// @title PQGasTest — Gas measurement tests for PQ-EVM operations
/// @notice Run with: forge test --gas-report --fork-url http://localhost:8545 -vv
/// @dev These tests measure real gas consumption of PQ operations on the PQ-EVM.
///      They require a running pq-reth node with the ML-DSA precompile at 0x0100
///      and the PQHASH opcode (0x21) enabled.
contract PQGasTest is Test {
    using PQVerify for bytes32;

    address pqhashHelper;

    function setUp() public {
        // Deploy the PQHASH helper contract (runtime: 365f5f37365f215f5260205ff3)
        bytes memory initCode = hex"600d80600e5f395ff3365f5f37365f215f5260205ff3";
        address deployed;
        assembly {
            deployed := create(0, add(initCode, 32), mload(initCode))
        }
        pqhashHelper = deployed;
    }

    // ─── ML-DSA Precompile Gas Tests ─────────────────────────────────────────

    /// @notice Measure gas for calling ML-DSA verify precompile with valid-length input
    function test_gasPrecompileCall_validLength() public {
        bytes memory sig = new bytes(3309);
        bytes memory pk = new bytes(1952);
        bytes32 msgHash = keccak256("test message");
        
        bytes memory input = abi.encodePacked(msgHash, sig, pk);
        
        uint256 gasBefore = gasleft();
        (bool success, bytes memory result) = address(0x0100).staticcall(input);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("ML-DSA verify precompile gas (valid-length)", gasBefore - gasAfter);
        
        assertTrue(success, "precompile call should not revert");
        assertEq(result.length, 32, "should return 32 bytes");
    }

    /// @notice Measure gas for precompile with invalid-length input (short-circuit)
    function test_gasPrecompileCall_invalidLength() public {
        bytes memory input = hex"deadbeef"; // too short
        
        uint256 gasBefore = gasleft();
        (bool success, bytes memory result) = address(0x0100).staticcall(input);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("ML-DSA verify precompile gas (invalid input)", gasBefore - gasAfter);
        
        assertTrue(success, "precompile should not revert on bad input");
    }

    /// @notice Measure gas for PQVerify library verify() function
    function test_gasPQVerifyLibrary() public {
        bytes memory sig = new bytes(3309);
        bytes memory pk = new bytes(1952);
        bytes32 msgHash = keccak256("test message");
        
        uint256 gasBefore = gasleft();
        bool valid = PQVerify.verify(msgHash, sig, pk);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("PQVerify.verify() library gas", gasBefore - gasAfter);
        
        // With dummy sig/pk, should return false (invalid signature)
        assertFalse(valid);
    }

    // ─── PQHASH Opcode Gas Tests ────────────────────────────────────────────

    /// @notice Measure gas for PQHASH (SHAKE-256) via helper contract — 32 bytes
    function test_gasPQHash_32bytes() public {
        if (pqhashHelper == address(0)) return;
        
        bytes memory data = abi.encodePacked(uint256(0x1234));
        
        uint256 gasBefore = gasleft();
        (bool success, bytes memory result) = pqhashHelper.staticcall(data);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("PQHASH (SHAKE-256) 32 bytes gas", gasBefore - gasAfter);
        
        assertTrue(success, "PQHASH helper should succeed");
        assertEq(result.length, 32, "should return 32-byte hash");
    }

    /// @notice Measure gas for PQHASH — 256 bytes
    function test_gasPQHash_256bytes() public {
        if (pqhashHelper == address(0)) return;
        
        bytes memory data = new bytes(256);
        for (uint i = 0; i < 256; i++) {
            data[i] = bytes1(uint8(i));
        }
        
        uint256 gasBefore = gasleft();
        (bool success,) = pqhashHelper.staticcall(data);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("PQHASH (SHAKE-256) 256 bytes gas", gasBefore - gasAfter);
        assertTrue(success);
    }

    /// @notice Measure gas for PQHASH — 1952 bytes (public key size)
    function test_gasPQHash_1952bytes() public {
        if (pqhashHelper == address(0)) return;
        
        bytes memory data = new bytes(1952);
        
        uint256 gasBefore = gasleft();
        (bool success,) = pqhashHelper.staticcall(data);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("PQHASH (SHAKE-256) 1952 bytes (pk size) gas", gasBefore - gasAfter);
        assertTrue(success);
    }

    // ─── Keccak-256 Comparison ───────────────────────────────────────────────

    /// @notice Measure gas for native keccak256 — 32 bytes (baseline comparison)
    function test_gasKeccak256_32bytes() public {
        bytes memory data = abi.encodePacked(uint256(0x1234));
        
        uint256 gasBefore = gasleft();
        bytes32 h = keccak256(data);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("Keccak-256 32 bytes gas", gasBefore - gasAfter);
        // Use h to prevent optimizer removal
        assertTrue(h != bytes32(0) || h == bytes32(0));
    }

    /// @notice Measure gas for native keccak256 — 256 bytes
    function test_gasKeccak256_256bytes() public {
        bytes memory data = new bytes(256);
        for (uint i = 0; i < 256; i++) {
            data[i] = bytes1(uint8(i));
        }
        
        uint256 gasBefore = gasleft();
        bytes32 h = keccak256(data);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("Keccak-256 256 bytes gas", gasBefore - gasAfter);
        assertTrue(h != bytes32(0) || h == bytes32(0));
    }

    /// @notice Measure gas for native keccak256 — 1952 bytes (pk size comparison)
    function test_gasKeccak256_1952bytes() public {
        bytes memory data = new bytes(1952);
        
        uint256 gasBefore = gasleft();
        bytes32 h = keccak256(data);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("Keccak-256 1952 bytes gas", gasBefore - gasAfter);
        assertTrue(h != bytes32(0) || h == bytes32(0));
    }

    // ─── Contract Deployment Gas ─────────────────────────────────────────────

    /// @notice Measure gas for deploying the PQHASH helper contract
    function test_gasDeployPQHashHelper() public {
        bytes memory initCode = hex"600d80600e5f395ff3365f5f37365f215f5260205ff3";
        
        uint256 gasBefore = gasleft();
        address deployed;
        assembly {
            deployed := create(0, add(initCode, 32), mload(initCode))
        }
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("Deploy PQHASH helper gas", gasBefore - gasAfter);
        assertTrue(deployed != address(0), "deploy should succeed");
    }

    // ─── ETH Transfer Gas ────────────────────────────────────────────────────

    /// @notice Measure gas for a simple ETH transfer
    function test_gasSimpleTransfer() public {
        address recipient = address(0xBEEF);
        vm.deal(address(this), 1 ether);
        
        uint256 gasBefore = gasleft();
        (bool success,) = recipient.call{value: 0.01 ether}("");
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("Simple ETH transfer gas (internal call)", gasBefore - gasAfter);
        assertTrue(success);
    }

    // ─── Memory/Calldata Cost for Large PQ Data ──────────────────────────────

    /// @notice Measure gas for ABI-encoding a PQ-sized payload (5293 bytes)
    function test_gasEncodePQPayload() public {
        bytes memory sig = new bytes(3309);
        bytes memory pk = new bytes(1952);
        bytes32 msgHash = keccak256("test");
        
        uint256 gasBefore = gasleft();
        bytes memory payload = abi.encodePacked(msgHash, sig, pk);
        uint256 gasAfter = gasleft();
        
        emit log_named_uint("ABI encode PQ payload (5293B) gas", gasBefore - gasAfter);
        assertEq(payload.length, 5293);
    }

    // Allow receiving ETH
    receive() external payable {}
}
