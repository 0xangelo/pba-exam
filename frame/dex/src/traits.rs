use sp_runtime::DispatchError;

/// For querying results of swaps without modifying storage. Doubles as a price oracle.
pub trait SimulateSwap {
    type AmmId;
    type AssetType;
    type Balance;

    /// Compute the amount of the opposite asset one would get if sending `amount` of `asset_type`
    /// to the AMM corresponding to `amm_id`.
    ///
    /// Takes slippage and fees into account, yield the net amount of asset that would be returned
    /// by the swap operation.
    fn simulate_swap(
        amm_id: Self::AmmId,
        asset_type: Self::AssetType,
        amount: Self::Balance,
    ) -> Result<Self::Balance, DispatchError>;
}
