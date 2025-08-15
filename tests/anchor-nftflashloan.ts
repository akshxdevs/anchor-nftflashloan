import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorNftflashloan } from "../target/types/anchor_nftflashloan";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import CryptoJS from "crypto-js";
describe("anchor-nftflashloan", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace
    .anchorNftflashloan as Program<AnchorNftflashloan>;
  const admin = provider.wallet as anchor.Wallet;
  const whiteList: PublicKey[] = [
    Keypair.generate().publicKey,
    Keypair.generate().publicKey,
    Keypair.generate().publicKey,
  ];

  function hashLeaf(mint: PublicKey): string {
    return CryptoJS.SHA256(mint.toBase58()).toString(CryptoJS.enc.Hex);
  }
  const leaves = whiteList.map(hashLeaf);
  function merkleRoot(leaves: string[]): string {
    let level = [...leaves];

    while (level.length > 1) {
      const nextLevel: string[] = [];
      for (let i = 0; i < level.length; i += 2) {
        const left = level[i];
        const right = i + 1 < level.length ? level[i + 1] : level[i];
        const combined = CryptoJS.SHA256(left + right).toString(
          CryptoJS.enc.Hex
        );
        nextLevel.push(combined);
      }
      level = nextLevel;
    }

    return level[0]; // hex string root
  }

  const root = merkleRoot(leaves);
  console.log("Merkle root:", root);
  function hexToBytes(hex: string): Uint8Array {
    const bytes = new Uint8Array(hex.length / 2);
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
    }
    return bytes;
  }
  const merkleRootBytes = hexToBytes(root);
  it("Is initialized!", async () => {
    const tx = await program.methods
      .initialize(100, Array.from(merkleRootBytes))
      .accounts({
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });
});
