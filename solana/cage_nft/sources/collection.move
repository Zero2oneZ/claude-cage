/// Collection — a shared object that groups NFTs.
/// Shared objects require consensus on Sui (unlike owned objects).
/// This is the right model for collections where multiple parties mint into it.
#[allow(lint(self_transfer))]
module cage_nft::collection {
    use std::string::String;
    use sui::event;

    /// A collection is a shared object — anyone with a reference can read it,
    /// but mutations require consensus.
    public struct Collection has key, store {
        id: UID,
        name: String,
        description: String,
        creator: address,
        max_supply: u64,
        current_supply: u64,
        /// Base URI for off-chain metadata
        base_uri: String,
    }

    /// Capability that proves you're the collection creator.
    /// Only the creator gets this — it's a resource, can't be copied.
    public struct CollectionCap has key, store {
        id: UID,
        collection_id: ID,
    }

    // ── Events ──────────────────────────────────────────────

    public struct CollectionCreated has copy, drop {
        collection_id: ID,
        name: String,
        creator: address,
        max_supply: u64,
    }

    // ── Public functions ────────────────────────────────────

    /// Create a new collection. Returns a capability to the creator.
    /// The collection is shared — visible to everyone on-chain.
    public fun create(
        name: String,
        description: String,
        max_supply: u64,
        base_uri: String,
        ctx: &mut TxContext,
    ) {
        let sender = ctx.sender();
        let collection = Collection {
            id: object::new(ctx),
            name,
            description,
            creator: sender,
            max_supply,
            current_supply: 0,
            base_uri,
        };

        let cap = CollectionCap {
            id: object::new(ctx),
            collection_id: object::id(&collection),
        };

        event::emit(CollectionCreated {
            collection_id: object::id(&collection),
            name: collection.name,
            creator: sender,
            max_supply,
        });

        // Share the collection — makes it a shared object (consensus required)
        transfer::public_share_object(collection);
        // Transfer the cap — only the creator holds this
        transfer::public_transfer(cap, sender);
    }

    /// Increment supply counter. Called by minting functions.
    /// Requires the CollectionCap — only the creator can authorize mints.
    public fun increment_supply(
        collection: &mut Collection,
        _cap: &CollectionCap,
    ) {
        assert!(collection.current_supply < collection.max_supply, 0);
        collection.current_supply = collection.current_supply + 1;
    }

    // ── View functions ──────────────────────────────────────

    public fun name(c: &Collection): &String { &c.name }
    public fun current_supply(c: &Collection): u64 { c.current_supply }
    public fun max_supply(c: &Collection): u64 { c.max_supply }
    public fun base_uri(c: &Collection): &String { &c.base_uri }
}
