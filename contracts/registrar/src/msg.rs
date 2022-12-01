use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Decimal};

#[cw_serde]
pub struct InstantiateMsg {
    pub name_nft_addr: String,
    pub verifier_pubkeys: Vec<Binary>,
    pub verification_threshold: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// To claim name, sender needs to gather signatures of `verifying_msg` form `verifiers`
    /// number of signatures must pass `verification_threshold` in order to proceed with minting
    /// and owning the name NFT
    Claim {
        /// Name to be minted as NFT
        name: String,

        /// String representation of [`VerifyingMsg`] that is used for
        /// generating verification signature
        verifying_msg: String,

        /// Vec of all verfications, which contains both signature
        /// and pubkey that use for that signature.
        verifications: Vec<Verification>,

        /// icns name of the referer, tracked for future incentivization
        referral: Option<String>,
    },
    UpdateVerifierPubkeys {
        add: Vec<Binary>,
        remove: Vec<Binary>,
    },
    SetVerificationThreshold {
        threshold: Decimal,
    },
    SetNameNFTAddress {
        name_nft_address: String,
    },
}
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(VerifierPubKeysResponse)]
    VerifierPubKeys {},

    #[returns(VerificationThresholdResponse)]
    VerificationThreshold {},

    #[returns(NameNFTAddressResponse)]
    NameNFTAddress {},

    #[returns(GetReferralCountResponse)]
    GetReferralCount { user_name: String },
}

#[cw_serde]
pub struct VerifierPubKeysResponse {
    pub verifier_pubkeys: Vec<Binary>,
}
#[cw_serde]
pub struct VerificationThresholdResponse {
    pub verification_threshold_percentage: Decimal,
}

#[cw_serde]
pub struct NameNFTAddressResponse {
    pub name_nft_address: String,
}

#[cw_serde]
pub struct GetReferralCountResponse {
    pub admins: Vec<String>,
}

#[cw_serde]
pub struct Verification {
    pub public_key: Binary,
    pub signature: Binary,
}

#[cw_serde]
pub struct VerifyingMsg {
    pub name: String,
    pub claimer: String,
    pub contract_address: String,
    pub chain_id: String,
}
