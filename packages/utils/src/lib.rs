/*!
This is a collection of common types shared among many specs.
For example [`Expiration`], which is embedded in many places.

Types should only be added here after they are duplicated in
a second contract, not "because we might need it"
*/

mod balance;
mod event;
mod expiration;
mod migrate;
mod pagination;
mod parse_reply;
mod payment;
mod scheduled;
mod threshold;

pub use migrate::ensure_from_older_version;
pub use pagination::{
    calc_range_end, calc_range_start, calc_range_start_string, maybe_addr, maybe_canonical,
};
pub use parse_reply::{
    parse_execute_response_data, parse_instantiate_response_data, parse_reply_execute_data,
    parse_reply_instantiate_data, MsgExecuteContractResponse, MsgInstantiateContractResponse,
    ParseReplyError,
};
pub use payment::{may_pay, must_pay, nonpayable, one_coin, PaymentError};
pub use threshold::{Threshold, ThresholdError, ThresholdResponse};

pub use crate::balance::NativeBalance;
pub use crate::event::Event;
pub use crate::expiration::{Duration, Expiration, DAY, HOUR, WEEK};
pub use crate::scheduled::Scheduled;
