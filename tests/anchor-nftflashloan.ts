import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorNftflashloan } from "../target/types/anchor_nftflashloan";
import { Keypair, PublicKey, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
import CryptoJS from "crypto-js";

describe("anchor-nftflashloan", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace
    .anchorNftflashloan as Program<AnchorNftflashloan>;
  const admin = provider.wallet as anchor.Wallet;
  
  // Test accounts
  const user = Keypair.generate();
  const liquidityMint = Keypair.generate();
  const nftMint = Keypair.generate();
  
  // PDAs
  let configPda: PublicKey;
  let vaultStatePda: PublicKey;
  let vaultAuthorityPda: PublicKey;
  let escrowPda: PublicKey;
  let escrowAuthorityPda: PublicKey;
  
  // Token accounts
  let userNftAta: PublicKey;
  let escrowNftAta: PublicKey;
  let vaultAta: PublicKey;
  let borrowerAta: PublicKey;
  
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
        const right = i + 1 < level.length ? level[i] : level[i];
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

  before(async () => {
    // Airdrop SOL to user for transaction fees
    const signature = await provider.connection.requestAirdrop(user.publicKey, 2 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(signature);
    
    // Create token mints
    await createMint(
      provider.connection,
      admin.payer,
      admin.publicKey,
      admin.publicKey,
      0, // decimals
      liquidityMint
    );
    
    await createMint(
      provider.connection,
      admin.payer,
      admin.publicKey,
      admin.publicKey,
      0, // decimals for NFT
      nftMint
    );
    
    // Create user NFT ATA
    userNftAta = await createAccount(
      provider.connection,
      admin.payer,
      nftMint.publicKey,
      user.publicKey
    );
    
    // Mint 1 NFT to user
    await mintTo(
      provider.connection,
      admin.payer,
      nftMint.publicKey,
      userNftAta,
      admin.payer,
      1
    );
    
    // Create borrower ATA
    borrowerAta = await createAccount(
      provider.connection,
      admin.payer,
      liquidityMint.publicKey,
      user.publicKey
    );
  });

  it("Is initialized!", async () => {
    // Find config PDA
    [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), admin.publicKey.toBuffer()],
      program.programId
    );
    
    const tx = await program.methods
      .initialize(100, Array.from(merkleRootBytes))
      .accounts({
        config: configPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("Initialize transaction signature:", tx);
    
    // Verify config was created
    const configAccount = await program.account.config.fetch(configPda);
    console.log("Config admin:", configAccount.admin.toString());
    console.log("Config fee BPS:", configAccount.feeBps);
    console.log("Config paused:", configAccount.paused);
  });

  it("Can initialize vault", async () => {
    // Find vault PDAs
    [vaultStatePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_state"), liquidityMint.publicKey.toBuffer()],
      program.programId
    );
    
    [vaultAuthorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_authority"), vaultStatePda.toBuffer()],
      program.programId
    );
    
    const tx = await program.methods
      .initVault()
      .accounts({
        vaultState: vaultStatePda,
        vaultAuthority: vaultAuthorityPda,
        config: configPda,
        liquidityMint: liquidityMint.publicKey,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("Init vault transaction signature:", tx);
    
    // Verify vault was created
    const vaultAccount = await program.account.vaultState.fetch(vaultStatePda);
    console.log("Vault mint:", vaultAccount.mint.toString());
    console.log("Vault config:", vaultAccount.config.toString());
    console.log("Vault in flash:", vaultAccount.inFlash);
  });

  it("Can set fee basis points", async () => {
    const newFeeBps = 200;
    
    const tx = await program.methods
      .setFeeBps(newFeeBps)
      .accounts({
        config: configPda,
        admin: admin.publicKey,
      })
      .rpc();
    console.log("Set fee BPS transaction signature:", tx);
    
    // Verify fee was updated
    const configAccount = await program.account.config.fetch(configPda);
    console.log("Updated fee BPS:", configAccount.feeBps);
  });

  it("Can set merkle root", async () => {
    const newMerkleRoot = Array.from(Keypair.generate().publicKey.toBytes());
    
    const tx = await program.methods
      .setMerkleRoot(newMerkleRoot)
      .accounts({
        config: configPda,
        admin: admin.publicKey,
      })
      .rpc();
    console.log("Set merkle root transaction signature:", tx);
    
    // Verify merkle root was updated
    const configAccount = await program.account.config.fetch(configPda);
    console.log("Updated merkle root:", configAccount.merkleRoot);
  });

  it("Can set paused state", async () => {
    const tx = await program.methods
      .setPaused(true)
      .accounts({
        config: configPda,
        admin: admin.publicKey,
      })
      .rpc();
    console.log("Set paused transaction signature:", tx);
    
    // Verify paused state was updated
    const configAccount = await program.account.config.fetch(configPda);
    console.log("Program paused:", configAccount.paused);
    
    // Unpause the program
    const tx2 = await program.methods
      .setPaused(false)
      .accounts({
        config: configPda,
        admin: admin.publicKey,
      })
      .rpc();
    console.log("Unpause transaction signature:", tx2);
    
    // Verify program is unpaused
    const configAccount2 = await program.account.config.fetch(configPda);
    console.log("Program unpaused:", configAccount2.paused);
  });
});
