import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorNftflashloan } from "../target/types/anchor_nftflashloan";

describe("anchor-nftflashloan", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.anchorNftflashloan as Program<AnchorNftflashloan>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
