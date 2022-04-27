#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, LogResponse, QueryMsg};
use crate::state::{LogEntry, LOG, PROCESSED_MSG};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    LOG.save(deps.storage, &vec![])?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;

    match msg {
        Touch {} => execute_touch(deps, info),
        Fail {} => execute_fail(deps, info),
        Forward {
            addr,
            msg,
            marker,
            catch_success,
            catch_failure,
            fail_reply,
        } => execute_forward(
            deps,
            info,
            addr,
            msg,
            marker,
            catch_success,
            catch_failure,
            fail_reply,
        ),
        Clear {} => execute_clear(deps),
        Reset {} => execute_reset(deps),
    }
}

pub fn execute_touch(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    LOG.update(deps.storage, |mut log| {
        let mut last = log.iter().last().cloned().unwrap_or_default();
        last.push(LogEntry {
            sender: info.sender,
            msg: ExecuteMsg::Touch {},
            reply: false,
            marker: None,
        });
        log.push(last);
        Ok::<_, ContractError>(log)
    })?;

    Ok(Response::new())
}

pub fn execute_fail(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Actually never would be stored as we immediately fail, but it is here to demonstrate
    // this behaviour
    LOG.update(deps.storage, |mut log| {
        let mut last = log.iter().last().cloned().unwrap_or_default();
        last.push(LogEntry {
            sender: info.sender,
            msg: ExecuteMsg::Fail {},
            reply: false,
            marker: None,
        });
        log.push(last);
        Ok::<_, ContractError>(log)
    })?;

    Err(ContractError::Fail {})
}

#[allow(clippy::too_many_arguments)]
pub fn execute_forward(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
    msg: Binary,
    marker: u64,
    catch_success: bool,
    catch_failure: bool,
    fail_reply: bool,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::Forward {
        addr: addr.clone(),
        msg,
        marker,
        catch_success,
        catch_failure,
        fail_reply,
    };

    PROCESSED_MSG.save(deps.storage, &msg)?;
    LOG.update(deps.storage, |mut log| {
        let mut last = log.iter().last().cloned().unwrap_or_default();
        last.push(LogEntry {
            sender: info.sender,
            msg: msg.clone(),
            reply: false,
            marker: Some(marker),
        });
        log.push(last);
        Ok::<_, ContractError>(log)
    })?;

    let msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: addr,
        msg: to_binary(&msg)?,
        funds: vec![],
    }
    .into();

    let msg = match (catch_success, catch_failure) {
        (false, false) => SubMsg::new(msg),
        (true, false) => SubMsg::reply_on_success(msg, marker),
        (false, true) => SubMsg::reply_on_error(msg, marker),
        (true, true) => SubMsg::reply_always(msg, marker),
    };

    let resp = Response::new().add_submessage(msg);
    Ok(resp)
}

pub fn execute_clear(deps: DepsMut) -> Result<Response, ContractError> {
    // Actually never would be stored as we immediately fail, but it is here to demonstrate
    // this behaviour
    LOG.update(deps.storage, |mut log| {
        log.push(vec![]);
        Ok::<_, ContractError>(log)
    })?;

    Ok(Response::new())
}

pub fn execute_reset(deps: DepsMut) -> Result<Response, ContractError> {
    LOG.save(deps.storage, &vec![])?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Log { depth } => to_binary(&query_log(deps, depth)?),
    }
}

pub fn query_log(deps: Deps, depth: Option<u64>) -> StdResult<LogResponse> {
    let mut log = LOG.load(deps.storage)?;
    if let Some(depth) = depth {
        log = log[log.len() - (depth as usize)..].into();
    }

    Ok(LogResponse { log })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let msg = PROCESSED_MSG.load(deps.storage)?;
    let fail = matches!(msg, ExecuteMsg::Forward { fail_reply, .. } if fail_reply);

    LOG.update(deps.storage, |mut log| {
        let mut last = log.iter().last().cloned().unwrap_or_default();
        last.push(LogEntry {
            sender: env.contract.address,
            msg,
            reply: true,
            marker: Some(reply.id),
        });
        log.push(last);
        Ok::<_, ContractError>(log)
    })?;

    if fail {
        Err(ContractError::Fail {})
    } else {
        Ok(Response::new())
    }
}
