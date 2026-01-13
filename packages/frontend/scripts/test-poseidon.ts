/**
 * Test Poseidon Contract Directly
 *
 * This script tests if the Poseidon contract is working correctly
 * Run with: npx tsx scripts/test-poseidon.ts
 */

import { createPublicClient, http, type Hex } from "viem";
import { sepolia } from "viem/chains";
import { mantleSepoliaTestnet } from "@/lib/web3/chains";

const POSEIDON_ABI = [
  {
    type: "function",
    name: "poseidon",
    stateMutability: "pure",
    inputs: [
      {
        name: "inputs",
        type: "bytes32[4]",
        internalType: "bytes32[4]",
      },
    ],
    outputs: [
      {
        name: "",
        type: "bytes32",
        internalType: "bytes32",
      },
    ],
  },
] as const;

const ETHEREUM_POSEIDON = "0x5d3efc46ddba9f083b571a64210B976E06e6d7B2" as Hex;
const MANTLE_POSEIDON = "0x8EA86eD4317AF92f73E5700eB9b93A72dE62f3B1" as Hex;

async function testPoseidonContract(
  chainName: string,
  rpcUrl: string,
  poseidonAddress: Hex
) {
  console.log(`\n🔍 Testing Poseidon contract on ${chainName}...`);
  console.log(`Address: ${poseidonAddress}`);
  console.log(`RPC: ${rpcUrl}`);

  try {
    const client = createPublicClient({
      transport: http(rpcUrl),
    });

    // Test with some sample inputs
    const testInputs: [Hex, Hex, Hex, Hex] = [
      "0x1234567890123456789012345678901234567890123456789012345678901234",
      "0x2345678901234567890123456789012345678901234567890123456789012345",
      "0x0000000000000000000000000000000000000000000000000000000000989680", // 10000000 wei (10 USDC)
      "0x0000000000000000000000000000000000000000000000000000000000000001", // chain ID 1
    ];

    console.log("\n📝 Test inputs:");
    console.log("  secret:     ", testInputs[0]);
    console.log("  nullifier:  ", testInputs[1]);
    console.log("  amount:     ", testInputs[2]);
    console.log("  chainId:    ", testInputs[3]);

    const result = await client.readContract({
      address: poseidonAddress,
      abi: POSEIDON_ABI,
      functionName: "poseidon",
      args: [testInputs],
    });

    console.log("\n✅ Poseidon contract call succeeded!");
    console.log("Result:", result);

    if (result === "0x0000000000000000000000000000000000000000000000000000000000000000") {
      console.log("\n❌ WARNING: Contract returned all zeros!");
      console.log("This indicates the contract is not working correctly.");
    } else {
      console.log("\n✅ Contract is working correctly (non-zero result)");
    }

    return result;
  } catch (error) {
    console.log("\n❌ Poseidon contract call FAILED!");
    console.error("Error:", error);
    throw error;
  }
}

async function main() {
  console.log("=".repeat(60));
  console.log("🧪 Poseidon Contract Diagnostic Test");
  console.log("=".repeat(60));

  // Test Ethereum Sepolia
  await testPoseidonContract(
    "Ethereum Sepolia",
    "https://eth-sepolia.g.alchemy.com/v2/hLuJpRL615TJAoKT3N7IHuJlgAQCzpe4",
    ETHEREUM_POSEIDON
  );

  // Test Mantle Sepolia
  await testPoseidonContract(
    "Mantle Sepolia",
    "https://mantle-sepolia.g.alchemy.com/v2/hLuJpRL615TJAoKT3N7IHuJlgAQCzpe4",
    MANTLE_POSEIDON
  );

  console.log("\n" + "=".repeat(60));
  console.log("✅ All tests completed");
  console.log("=".repeat(60) + "\n");
}

main().catch((error) => {
  console.error("\n💥 Fatal error:", error);
  process.exit(1);
});
