#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::Order::Ascending;

use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response, StdResult,
    WasmQuery,
};
use cw2::set_contract_version;
use subtle_encoding::bech32;

use crate::crypto::adr36_verification;
use crate::error::ContractError;
use crate::msg::{
    AddressHash, AddressResponse, AddressesResponse, Adr36Info, ExecuteMsg, InstantiateMsg,
    PrimaryNameResponse, QueryMsg,
};
use crate::state::{records, Config, CONFIG, PRIMARY_NAME, SIGNATURE};
use cw721::OwnerOfResponse;
use icns_name_nft::msg::{AdminResponse, QueryMsg as QueryMsgName};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:icns-resolver";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let name_address = deps.api.addr_validate(&msg.name_address)?;

    let cfg = Config { name_address };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetRecord {
            name,
            bech32_prefix,
            adr36_info,
            signature_salt,
        } => execute_set_record(
            deps,
            env,
            info,
            name,
            bech32_prefix,
            adr36_info,
            signature_salt.u128(),
        ),
        ExecuteMsg::SetPrimary { name, bech32_address } => execute_set_primary(deps, info, name, bech32_address),
    }
}

pub fn execute_set_record(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    bech32_prefix: String,
    adr36_info: Adr36Info,
    signature_salt: u128,
) -> Result<Response, ContractError> {
    // check if the msg sender is a registrar or admin. If not, return err
    let is_admin = is_admin(deps.as_ref(), info.sender.to_string())?;
    let is_owner_nft = is_owner(deps.as_ref(), name.clone(), info.sender.to_string())?;

    // if the sender is neither a registrar nor an admin, return error
    if !is_admin && !is_owner_nft {
        return Err(ContractError::Unauthorized {});
    }

    // check address hash method, currently only sha256 is supported
    if adr36_info.address_hash != AddressHash::SHA256 {
        return Err(ContractError::HashMethodNotSupported {});
    }

    // extract bech32 prefix from given address
    let bech32_prefix_decoded = bech32::decode(adr36_info.bech32_address.clone())
        .map_err(|_| ContractError::Bech32DecodingErr {
            addr: adr36_info.bech32_address.to_string(),
        })?
        .0;

    // first check if the user input for prefix + address is valid
    if bech32_prefix != bech32_prefix_decoded {
        return Err(ContractError::Bech32PrefixMismatch {
            prefix: bech32_prefix,
            addr: adr36_info.bech32_address,
        });
    }

    // do adr36 verification
    let chain_id = env.block.chain_id;
    let contract_address = env.contract.address.to_string();
    adr36_verification(
        deps.as_ref(),
        name.clone(),
        bech32_prefix.clone(),
        adr36_info.clone(),
        chain_id,
        contract_address,
        signature_salt,
    )?;

    // save record
    records().save(deps.storage, (&name, &bech32_prefix), &adr36_info.bech32_address.clone())?;

    // set name as primary name if it doesn't exists for this address yet
    let primary_name = PRIMARY_NAME.key(adr36_info.bech32_address);
    if primary_name.may_load(deps.storage)?.is_none() {
        primary_name.save(deps.storage, &name)?
    }

    // save signature to prevent replay attack
    SIGNATURE.save(deps.storage, adr36_info.signature.as_slice(), &true)?;

    Ok(Response::default())
}

fn execute_set_primary(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    bech32_address: String,
) -> Result<Response, ContractError> {
    if !is_owner(deps.as_ref(), name.clone(), info.sender.to_string())? {
        return Err(ContractError::Unauthorized {});
    }

    // extract bech32 prefix from given address
    let bech32_prefix_decoded = bech32::decode(bech32_address.clone())
    .map_err(|_| ContractError::Bech32DecodingErr {
        addr: bech32_address.to_string(),
    })?
    .0;

    // bech32 address needs to be already set in the records(for adr36 veficiation)
    // check in state if this is already set
    let bech32_address_stored = records().may_load(deps.storage, (&name, &bech32_prefix_decoded))?;
    if bech32_address_stored.is_none() || bech32_address_stored.unwrap() != bech32_address {
        return Err(ContractError::Bech32AddressNotSet {
            name: name.clone(),
            address: bech32_address.clone(),
        });
    }

    PRIMARY_NAME.save(deps.storage, bech32_address, &name)?;

    Ok(Response::new()
        .add_attribute("method", "set_primary")
        .add_attribute("name", name))
}

pub fn is_admin(deps: Deps, address: String) -> Result<bool, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let name_address = cfg.name_address;

    // query admin from icns-name-nft contract
    let query_msg = QueryMsgName::Admin {};
    let res: AdminResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: name_address.to_string(),
        msg: to_binary(&query_msg)?,
    }))?;

    Ok(res.admins.into_iter().any(|admin| admin.eq(&address)))
}

pub fn admin(deps: Deps) -> Result<Vec<String>, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let name_address = cfg.name_address;

    // query admin from icns-name-nft contract
    let query_msg = QueryMsgName::Admin {};
    let res: AdminResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: name_address.to_string(),
        msg: to_binary(&query_msg)?,
    }))?;

    Ok(res.admins)
}

pub fn is_owner(deps: Deps, username: String, sender: String) -> Result<bool, ContractError> {
    let response = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: CONFIG.load(deps.storage)?.name_address.to_string(),
        msg: to_binary(&QueryMsgName::OwnerOf {
            token_id: username,
            include_expired: None,
        })?,
    }));

    match response {
        Ok(OwnerOfResponse { owner, .. }) => Ok(owner == sender),
        Err(_) => Ok(false),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Addresses { name } => to_binary(&query_addresses(deps, env, name)?),
        QueryMsg::Address {
            name,
            bech32_prefix,
        } => to_binary(&query_address(deps, env, name, bech32_prefix)?),
        QueryMsg::Admin {} => to_binary(&query_admin(deps)?),
        QueryMsg::PrimaryName { address } => to_binary(&query_primary_name(deps, address)?),
        // TODO: add query to query directly using ICNS (e.g req: tony.eth)
    }
}

fn query_primary_name(deps: Deps, address: String) -> StdResult<PrimaryNameResponse> {
    Ok(PrimaryNameResponse {
        name: PRIMARY_NAME.load(deps.storage, address)?,
    })
}

fn query_addresses(deps: Deps, _env: Env, name: String) -> StdResult<AddressesResponse> {
    Ok(AddressesResponse {
        addresses: records()
            .prefix(&name)
            .range(deps.storage, None, None, Ascending)
            .collect::<StdResult<Vec<_>>>()?,
    })
}

fn query_address(
    deps: Deps,
    _env: Env,
    name: String,
    bech32_prefix: String,
) -> StdResult<AddressResponse> {
    Ok(AddressResponse {
        address: records().load(deps.storage, (&name, &bech32_prefix))?.to_string(),
    })
}

fn query_admin(deps: Deps) -> StdResult<AdminResponse> {
    // unwrap this
    let result = admin(deps);
    match result {
        Ok(admins) => Ok(AdminResponse { admins }),
        Err(_) => Ok(AdminResponse {
            admins: vec![String::from("")],
        }),
    }
}
