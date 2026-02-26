/// SYNTH Token — a Move resource representing Proof-of-Reasoning value.
///
/// This is where linear types shine. A SynthCoin:
///   - Cannot be copied (no `copy` ability) → no double-spending
///   - Cannot be dropped (no `drop` ability) → must be explicitly consumed
///   - Can only be minted by the treasury module → no fabrication
///
/// The compiler enforces conservation of value. If you split a coin,
/// the two halves must sum to the original. If you try to forget a coin,
/// the compiler rejects your code.
#[allow(deprecated_usage, lint(self_transfer))]
module cage_nft::synth_token {
    use sui::coin::{Self, Coin, TreasuryCap};
    use sui::url;

    /// One-time witness for the coin. The OTW pattern ensures this module
    /// can only create ONE type of coin, ever. The struct name must match
    /// the module name in uppercase.
    public struct SYNTH_TOKEN has drop {}

    /// Initialize the SYNTH coin type. Called exactly once at publish time.
    /// Creates the TreasuryCap (mint authority) and CoinMetadata (on-chain metadata).
    fun init(witness: SYNTH_TOKEN, ctx: &mut TxContext) {
        let (treasury_cap, metadata) = coin::create_currency<SYNTH_TOKEN>(
            witness,
            9,                                                      // decimals
            b"SYNTH",                                               // symbol
            b"SYNTH Token",                                         // name
            b"Proof-of-Reasoning value token for GentlyOS swarm",   // description
            option::some(url::new_unsafe_from_bytes(b"https://gentlyos.dev/synth-icon.png")),
            ctx,
        );

        // Freeze metadata — immutable on-chain, no one can change symbol/name/decimals
        transfer::public_freeze_object(metadata);

        // Transfer treasury cap to deployer — this is the ONLY way to mint SYNTH
        transfer::public_transfer(treasury_cap, ctx.sender());
    }

    // ── Mint / Burn (requires TreasuryCap) ──────────────────

    /// Mint SYNTH tokens. Only the holder of TreasuryCap can call this.
    /// In production: the PTC orchestrator module would hold the cap.
    public fun mint(
        treasury_cap: &mut TreasuryCap<SYNTH_TOKEN>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let coin = coin::mint(treasury_cap, amount, ctx);
        transfer::public_transfer(coin, recipient);
    }

    /// Burn SYNTH tokens. Reduces total supply.
    /// The coin resource is consumed — linear types guarantee it's gone.
    public fun burn(
        treasury_cap: &mut TreasuryCap<SYNTH_TOKEN>,
        coin: Coin<SYNTH_TOKEN>,
    ) {
        coin::burn(treasury_cap, coin);
    }

    // ── Utility ─────────────────────────────────────────────

    /// Split: conservation of value enforced by the Coin module's linear types.
    /// coin::split guarantees: original.value = returned.value + split_amount
    public fun split_and_transfer(
        coin: &mut Coin<SYNTH_TOKEN>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let split_coin = coin::split(coin, amount, ctx);
        transfer::public_transfer(split_coin, recipient);
    }
}
