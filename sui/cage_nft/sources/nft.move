/// Cage NFT — a Sui object-based NFT with on-chain metadata.
/// Demonstrates the resource model: NFTs are objects with `key` + `store`,
/// meaning they live on-chain, are owned, and can be transferred.
#[allow(lint(self_transfer))]
module cage_nft::nft {
    use std::string::String;
    use sui::event;

    /// The NFT itself — a Sui object (has `key`), transferable (has `store`).
    /// Cannot be copied or dropped — must be explicitly transferred or burned.
    public struct CageNFT has key, store {
        id: UID,
        name: String,
        description: String,
        image_url: String,
        /// Arbitrary attributes as key-value pairs
        creator: address,
        /// Collection this NFT belongs to (0x0 if standalone)
        collection_id: ID,
    }

    // ── Events ──────────────────────────────────────────────

    public struct NFTMinted has copy, drop {
        nft_id: ID,
        name: String,
        creator: address,
    }

    public struct NFTBurned has copy, drop {
        nft_id: ID,
        name: String,
    }

    // ── Public functions ────────────────────────────────────

    /// Mint a new NFT. The resource is transferred to the sender.
    /// Only the creator can call this — enforced by `ctx.sender()`.
    public fun mint(
        name: String,
        description: String,
        image_url: String,
        ctx: &mut TxContext,
    ) {
        let sender = ctx.sender();
        let nft = CageNFT {
            id: object::new(ctx),
            name,
            description,
            image_url,
            creator: sender,
            collection_id: object::id_from_address(@0x0),
        };

        event::emit(NFTMinted {
            nft_id: object::id(&nft),
            name: nft.name,
            creator: sender,
        });

        transfer::public_transfer(nft, sender);
    }

    /// Mint an NFT into a specific collection.
    public fun mint_to_collection(
        name: String,
        description: String,
        image_url: String,
        collection_id: ID,
        ctx: &mut TxContext,
    ) {
        let sender = ctx.sender();
        let nft = CageNFT {
            id: object::new(ctx),
            name,
            description,
            image_url,
            creator: sender,
            collection_id,
        };

        event::emit(NFTMinted {
            nft_id: object::id(&nft),
            name: nft.name,
            creator: sender,
        });

        transfer::public_transfer(nft, sender);
    }

    /// Burn an NFT — the resource is consumed (destructured), not dropped.
    /// This is the Move linear type guarantee: you MUST explicitly handle the resource.
    public fun burn(nft: CageNFT) {
        let CageNFT { id, name, description: _, image_url: _, creator: _, collection_id: _ } = nft;
        event::emit(NFTBurned {
            nft_id: id.to_inner(),
            name,
        });
        object::delete(id);
    }

    /// Transfer an NFT to another address.
    public fun transfer_nft(nft: CageNFT, recipient: address) {
        transfer::public_transfer(nft, recipient);
    }

    // ── View functions ──────────────────────────────────────

    public fun name(nft: &CageNFT): &String { &nft.name }
    public fun description(nft: &CageNFT): &String { &nft.description }
    public fun image_url(nft: &CageNFT): &String { &nft.image_url }
    public fun creator(nft: &CageNFT): address { nft.creator }
}
