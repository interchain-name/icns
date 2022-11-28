use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;

/// Message type for `instantiate` entry_point
// TODO: change this to array
#[cw_serde]
pub struct InstantiateMsg {
    pub name_nft_addr: String,
    pub verifier_pubkeys: Vec<String>,
    pub verification_threshold: Decimal,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    Claim {
        name: String,
        verifying_msg: String,
        // vec of `base64(secp256k1_sign(verifying_msg, verifier_key))`
        verifications: Vec<Verification>,
    },
    AddVerifier {
        verifier_addr: String,
    },
    RemoveVerifier {
        verifier_addr: String,
    },
    SetVerificationThreshold {
        threshold: Decimal,
    },
}
#[cw_serde]
pub struct QueryMsg;
#[cw_serde]
pub struct Verification {
    pub public_key: String,
    pub signature: String,
}

#[cw_serde]
pub struct VerifyingMsg {
    pub name: String,
    pub claimer: String,
    pub contract_address: String,
    pub chain_id: String,
}
