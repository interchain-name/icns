pub use crate::msg::{InstantiateMsg, QueryMsg};
use cosmwasm_std::Empty;
pub use cw721_base::{
    entry::{execute as _execute, query as _query},
    ContractError, Cw721Contract, ExecuteMsg as CW721BaseExecuteMsg, Extension,
    InstantiateMsg as Cw721BaseInstantiateMsg, MintMsg, MinterResponse,
};

pub mod msg;
pub mod query;
pub mod state;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:icns-name-ownership";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type ICNSNameContract<'a> = Cw721Contract<'a, Extension, Empty, Empty, Empty>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;
    use crate::msg::ExecuteMsg;
    use crate::query::{admin, transferable};
    use crate::state::{Config, CONFIG};
    use cosmwasm_std::{
        entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    };

    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let admin_addr: Addr = deps.api.addr_validate(&msg.admin)?;

        let config = Config {
            admin: admin_addr,
            transferable: msg.transferable,
        };

        CONFIG.save(deps.storage, &config)?;

        let cw721_base_instantiate_msg = Cw721BaseInstantiateMsg {
            name: msg.name,
            symbol: msg.symbol,
            minter: msg.minter,
        };

        ICNSNameContract::default().instantiate(
            deps.branch(),
            env,
            info,
            cw721_base_instantiate_msg,
        )?;

        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        Ok(Response::default()
            .add_attribute("contract_name", CONTRACT_NAME)
            .add_attribute("contract_version", CONTRACT_VERSION))
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, cw721_base::ContractError> {
        let config = CONFIG.load(deps.storage)?;
        match msg {
            ExecuteMsg::CW721Base(msg) => {
                let MinterResponse { minter } =
                    ICNSNameContract::default().minter(deps.as_ref())?;

                // TODO: do finer-grained control by calling each execute function directly.
                if config.admin == info.sender || minter == info.sender || config.transferable {
                    _execute(deps, env, info, msg)
                } else {
                    Err(ContractError::Unauthorized {})
                }
            }
            ExecuteMsg::ICNSName(msg) => match msg {
                msg::ICNSNameExecuteMsg::SetAdmin { admin } => {
                    if config.admin == info.sender {
                        CONFIG.update(deps.storage, |config| -> StdResult<_> {
                            Ok(Config {
                                admin: deps.api.addr_validate(&admin)?,
                                ..config
                            })
                        })?;
                        Ok(Response::new()
                            .add_attribute("method", "set_admin")
                            .add_attribute("admin", admin))
                    } else {
                        Err(ContractError::Unauthorized {})
                    }
                }
                msg::ICNSNameExecuteMsg::SetTransferrable {
                    transferrable: transferable,
                } => {
                    if config.admin == info.sender {
                        CONFIG.update(deps.storage, |config| -> StdResult<_> {
                            Ok(Config {
                                transferable,
                                ..config
                            })
                        })?;
                        Ok(Response::new()
                            .add_attribute("method", "set_transferrable")
                            .add_attribute("transferrable", transferable.to_string()))
                    } else {
                        Err(ContractError::Unauthorized {})
                    }
                }
            },
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Admin {} => to_binary(&admin(deps)?),
            QueryMsg::Transferrable {} => to_binary(&transferable(deps)?),
            _ => _query(deps, env, msg.into()),
        }
    }
}

#[cfg(test)]
mod tests;
