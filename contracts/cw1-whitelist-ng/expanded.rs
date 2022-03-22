#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2018::*;
#[macro_use]
extern crate std;
mod contract {
    use cosmwasm_std::{
        from_slice, Addr, Api, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
        StdError, StdResult,
    };
    use serde::de::DeserializeOwned;
    use crate::error::ContractError;
    use crate::interfaces::*;
    use crate::msg::{cw1, whitelist, AdminListResponse};
    use crate::state::{AdminList, Cw1WhitelistContract};
    use cw1::CanExecuteResponse;
    use cw2::set_contract_version;
    const CONTRACT_NAME: &str = "crates.io:cw1-whitelist";
    const CONTRACT_VERSION: &str = "0.11.1";
    pub fn validate_admins(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
        admins.iter().map(|addr| api.addr_validate(addr)).collect()
    }
    impl<T> Cw1WhitelistContract<T> {
        pub fn instantiate(
            &self,
            (deps, _env, _info): (DepsMut, Env, MessageInfo),
            admins: Vec<String>,
            mutable: bool,
        ) -> Result<Response<T>, ContractError> {
            set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
            let cfg = AdminList {
                admins: validate_admins(deps.api, &admins)?,
                mutable,
            };
            self.admin_list.save(deps.storage, &cfg)?;
            Ok(Response::new())
        }
        pub fn is_admin(&self, deps: Deps, addr: &str) -> Result<bool, ContractError> {
            let cfg = self.admin_list.load(deps.storage)?;
            Ok(cfg.is_admin(addr))
        }
        pub(crate) fn entry_execute(
            &self,
            deps: DepsMut,
            env: Env,
            info: MessageInfo,
            msg: &[u8],
        ) -> Result<Response<T>, ContractError>
        where
            T: DeserializeOwned,
        {
            let cw1_err = match from_slice::<Cw1ExecMsg<T>>(msg) {
                Ok(msg) => return msg.dispatch(deps, env, info, self),
                Err(err) => err,
            };
            let whitelist_err = match from_slice::<WhitelistExecMsg>(msg) {
                Ok(msg) => return msg.dispatch(deps, env, info, self),
                Err(err) => err,
            };
            let msg = {
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &[
                        "While parsing Cw1WhitelistExecMsg\n As Cw1ExecMsg: ",
                        "\n As WhitelistExecMsg: ",
                    ],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&cw1_err),
                        ::core::fmt::ArgumentV1::new_display(&whitelist_err),
                    ],
                ));
                res
            };
            let err = StdError::parse_err("Cw1WhitelistExecMsg", msg);
            Err(err.into())
        }
        pub(crate) fn entry_query(
            &self,
            deps: Deps,
            env: Env,
            msg: &[u8],
        ) -> Result<Binary, ContractError>
        where
            T: DeserializeOwned,
        {
            let cw1_err = match from_slice::<Cw1QueryMsg<T>>(msg) {
                Ok(msg) => return msg.dispatch(deps, env, self),
                Err(err) => err,
            };
            let whitelist_err = match from_slice::<WhitelistQueryMsg>(msg) {
                Ok(msg) => return msg.dispatch(deps, env, self),
                Err(err) => err,
            };
            let msg = {
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &[
                        "While parsing Cw1WhitelistQueryMsg\n As Cw1QueryMsg: ",
                        "\n As WhitelistQueryMsg: ",
                    ],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&cw1_err),
                        ::core::fmt::ArgumentV1::new_display(&whitelist_err),
                    ],
                ));
                res
            };
            let err = StdError::parse_err("Cw1WhitelistExecMsg", msg);
            Err(err.into())
        }
    }
    pub mod msg {
        use super::*;
        #[serde(rename_all = "snake_case")]
        pub struct InstantiateMsg {
            admins: Vec<String>,
            mutable: bool,
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl _serde::Serialize for InstantiateMsg {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    let mut __serde_state = match _serde::Serializer::serialize_struct(
                        __serializer,
                        "InstantiateMsg",
                        false as usize + 1 + 1,
                    ) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    };
                    match _serde::ser::SerializeStruct::serialize_field(
                        &mut __serde_state,
                        "admins",
                        &self.admins,
                    ) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    };
                    match _serde::ser::SerializeStruct::serialize_field(
                        &mut __serde_state,
                        "mutable",
                        &self.mutable,
                    ) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    };
                    _serde::ser::SerializeStruct::end(__serde_state)
                }
            }
        };
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for InstantiateMsg {
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    #[allow(non_camel_case_types)]
                    enum __Field {
                        __field0,
                        __field1,
                        __ignore,
                    }
                    struct __FieldVisitor;
                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter, "field identifier")
                        }
                        fn visit_u64<__E>(
                            self,
                            __value: u64,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                0u64 => _serde::__private::Ok(__Field::__field0),
                                1u64 => _serde::__private::Ok(__Field::__field1),
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                        fn visit_str<__E>(
                            self,
                            __value: &str,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                "admins" => _serde::__private::Ok(__Field::__field0),
                                "mutable" => _serde::__private::Ok(__Field::__field1),
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                        fn visit_bytes<__E>(
                            self,
                            __value: &[u8],
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                b"admins" => _serde::__private::Ok(__Field::__field0),
                                b"mutable" => _serde::__private::Ok(__Field::__field1),
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                    }
                    impl<'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(
                            __deserializer: __D,
                        ) -> _serde::__private::Result<Self, __D::Error>
                        where
                            __D: _serde::Deserializer<'de>,
                        {
                            _serde::Deserializer::deserialize_identifier(
                                __deserializer,
                                __FieldVisitor,
                            )
                        }
                    }
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<InstantiateMsg>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = InstantiateMsg;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "struct InstantiateMsg",
                            )
                        }
                        #[inline]
                        fn visit_seq<__A>(
                            self,
                            mut __seq: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::SeqAccess<'de>,
                        {
                            let __field0 = match match _serde::de::SeqAccess::next_element::<
                                Vec<String>,
                            >(&mut __seq)
                            {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            } {
                                _serde::__private::Some(__value) => __value,
                                _serde::__private::None => {
                                    return _serde::__private::Err(
                                        _serde::de::Error::invalid_length(
                                            0usize,
                                            &"struct InstantiateMsg with 2 elements",
                                        ),
                                    );
                                }
                            };
                            let __field1 =
                                match match _serde::de::SeqAccess::next_element::<bool>(&mut __seq)
                                {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                } {
                                    _serde::__private::Some(__value) => __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(
                                            _serde::de::Error::invalid_length(
                                                1usize,
                                                &"struct InstantiateMsg with 2 elements",
                                            ),
                                        );
                                    }
                                };
                            _serde::__private::Ok(InstantiateMsg {
                                admins: __field0,
                                mutable: __field1,
                            })
                        }
                        #[inline]
                        fn visit_map<__A>(
                            self,
                            mut __map: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::MapAccess<'de>,
                        {
                            let mut __field0: _serde::__private::Option<Vec<String>> =
                                _serde::__private::None;
                            let mut __field1: _serde::__private::Option<bool> =
                                _serde::__private::None;
                            while let _serde::__private::Some(__key) =
                                match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                }
                            {
                                match __key {
                                    __Field::__field0 => {
                                        if _serde::__private::Option::is_some(&__field0) {
                                            return _serde::__private::Err(
                                                <__A::Error as _serde::de::Error>::duplicate_field(
                                                    "admins",
                                                ),
                                            );
                                        }
                                        __field0 = _serde::__private::Some(
                                            match _serde::de::MapAccess::next_value::<Vec<String>>(
                                                &mut __map,
                                            ) {
                                                _serde::__private::Ok(__val) => __val,
                                                _serde::__private::Err(__err) => {
                                                    return _serde::__private::Err(__err);
                                                }
                                            },
                                        );
                                    }
                                    __Field::__field1 => {
                                        if _serde::__private::Option::is_some(&__field1) {
                                            return _serde::__private::Err(
                                                <__A::Error as _serde::de::Error>::duplicate_field(
                                                    "mutable",
                                                ),
                                            );
                                        }
                                        __field1 = _serde::__private::Some(
                                            match _serde::de::MapAccess::next_value::<bool>(
                                                &mut __map,
                                            ) {
                                                _serde::__private::Ok(__val) => __val,
                                                _serde::__private::Err(__err) => {
                                                    return _serde::__private::Err(__err);
                                                }
                                            },
                                        );
                                    }
                                    _ => {
                                        let _ = match _serde::de::MapAccess::next_value::<
                                            _serde::de::IgnoredAny,
                                        >(
                                            &mut __map
                                        ) {
                                            _serde::__private::Ok(__val) => __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        };
                                    }
                                }
                            }
                            let __field0 = match __field0 {
                                _serde::__private::Some(__field0) => __field0,
                                _serde::__private::None => {
                                    match _serde::__private::de::missing_field("admins") {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    }
                                }
                            };
                            let __field1 = match __field1 {
                                _serde::__private::Some(__field1) => __field1,
                                _serde::__private::None => {
                                    match _serde::__private::de::missing_field("mutable") {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    }
                                }
                            };
                            _serde::__private::Ok(InstantiateMsg {
                                admins: __field0,
                                mutable: __field1,
                            })
                        }
                    }
                    const FIELDS: &'static [&'static str] = &["admins", "mutable"];
                    _serde::Deserializer::deserialize_struct(
                        __deserializer,
                        "InstantiateMsg",
                        FIELDS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<InstantiateMsg>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
            }
        };
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for InstantiateMsg {
            #[inline]
            fn clone(&self) -> InstantiateMsg {
                match *self {
                    InstantiateMsg {
                        admins: ref __self_0_0,
                        mutable: ref __self_0_1,
                    } => InstantiateMsg {
                        admins: ::core::clone::Clone::clone(&(*__self_0_0)),
                        mutable: ::core::clone::Clone::clone(&(*__self_0_1)),
                    },
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for InstantiateMsg {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match *self {
                    InstantiateMsg {
                        admins: ref __self_0_0,
                        mutable: ref __self_0_1,
                    } => {
                        let debug_trait_builder =
                            &mut ::core::fmt::Formatter::debug_struct(f, "InstantiateMsg");
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "admins",
                            &&(*__self_0_0),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "mutable",
                            &&(*__self_0_1),
                        );
                        ::core::fmt::DebugStruct::finish(debug_trait_builder)
                    }
                }
            }
        }
        impl ::core::marker::StructuralPartialEq for InstantiateMsg {}
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for InstantiateMsg {
            #[inline]
            fn eq(&self, other: &InstantiateMsg) -> bool {
                match *other {
                    InstantiateMsg {
                        admins: ref __self_1_0,
                        mutable: ref __self_1_1,
                    } => match *self {
                        InstantiateMsg {
                            admins: ref __self_0_0,
                            mutable: ref __self_0_1,
                        } => (*__self_0_0) == (*__self_1_0) && (*__self_0_1) == (*__self_1_1),
                    },
                }
            }
            #[inline]
            fn ne(&self, other: &InstantiateMsg) -> bool {
                match *other {
                    InstantiateMsg {
                        admins: ref __self_1_0,
                        mutable: ref __self_1_1,
                    } => match *self {
                        InstantiateMsg {
                            admins: ref __self_0_0,
                            mutable: ref __self_0_1,
                        } => (*__self_0_0) != (*__self_1_0) || (*__self_0_1) != (*__self_1_1),
                    },
                }
            }
        }
        const _: () = {
            #[automatically_derived]
            #[allow(unused_braces)]
            impl schemars::JsonSchema for InstantiateMsg {
                fn schema_name() -> std::string::String {
                    "InstantiateMsg".to_owned()
                }
                fn json_schema(
                    gen: &mut schemars::gen::SchemaGenerator,
                ) -> schemars::schema::Schema {
                    {
                        let mut schema_object = schemars::schema::SchemaObject {
                            instance_type: Some(schemars::schema::InstanceType::Object.into()),
                            ..Default::default()
                        };
                        let object_validation = schema_object.object();
                        {
                            object_validation
                                .properties
                                .insert("admins".to_owned(), gen.subschema_for::<Vec<String>>());
                            if !<Vec<String> as schemars::JsonSchema>::_schemars_private_is_option()
                            {
                                object_validation.required.insert("admins".to_owned());
                            }
                        }
                        {
                            object_validation
                                .properties
                                .insert("mutable".to_owned(), gen.subschema_for::<bool>());
                            if !<bool as schemars::JsonSchema>::_schemars_private_is_option() {
                                object_validation.required.insert("mutable".to_owned());
                            }
                        }
                        schemars::schema::Schema::Object(schema_object)
                    }
                }
            };
        };
        impl InstantiateMsg {
            pub fn dispatch<T>(
                self,
                contract: Cw1WhitelistContract<T>,
                ctx: (
                    cosmwasm_std::DepsMut,
                    cosmwasm_std::Env,
                    cosmwasm_std::MessageInfo,
                ),
            ) -> Result<Response<T>, ContractError> {
                let Self { admins, mutable } = self;
                contract
                    .instantiate(ctx.into(), admins, mutable)
                    .map_err(Into::into)
            }
        }
    }
    impl<T> crate::interfaces::Cw1<T> for Cw1WhitelistContract<T>
    where
        T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
    {
        type Error = ContractError;
        fn execute(
            &self,
            (deps, _env, info): (DepsMut, Env, MessageInfo),
            msgs: Vec<CosmosMsg<T>>,
        ) -> Result<Response<T>, Self::Error> {
            if !self.is_admin(deps.as_ref(), info.sender.as_ref())? {
                Err(ContractError::Unauthorized {})
            } else {
                let res = Response::new()
                    .add_messages(msgs)
                    .add_attribute("action", "execute");
                Ok(res)
            }
        }
        fn can_execute(
            &self,
            (deps, _env): (Deps, Env),
            sender: String,
            _msg: CosmosMsg<T>,
        ) -> Result<CanExecuteResponse, Self::Error> {
            Ok(CanExecuteResponse {
                can_execute: self.is_admin(deps, &sender)?,
            })
        }
    }
    impl<T> crate::interfaces::Whitelist<T> for Cw1WhitelistContract<T>
    where
        T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
    {
        type Error = ContractError;
        fn freeze(
            &self,
            (deps, _env, info): (DepsMut, Env, MessageInfo),
        ) -> Result<Response<T>, Self::Error> {
            self.admin_list
                .update(deps.storage, |mut cfg| -> Result<_, ContractError> {
                    if !cfg.can_modify(info.sender.as_str()) {
                        Err(ContractError::Unauthorized {})
                    } else {
                        cfg.mutable = false;
                        Ok(cfg)
                    }
                })?;
            Ok(Response::new().add_attribute("action", "freeze"))
        }
        fn update_admins(
            &self,
            (deps, _env, info): (DepsMut, Env, MessageInfo),
            admins: Vec<String>,
        ) -> Result<Response<T>, Self::Error> {
            let api = deps.api;
            self.admin_list
                .update(deps.storage, |mut cfg| -> Result<_, ContractError> {
                    if !cfg.can_modify(info.sender.as_str()) {
                        Err(ContractError::Unauthorized {})
                    } else {
                        cfg.admins = validate_admins(api, &admins)?;
                        Ok(cfg)
                    }
                })?;
            Ok(Response::new().add_attribute("action", "update_admins"))
        }
        fn admin_list(&self, (deps, _env): (Deps, Env)) -> Result<AdminListResponse, Self::Error> {
            let cfg = self.admin_list.load(deps.storage)?;
            Ok(AdminListResponse {
                admins: cfg.admins.into_iter().map(|a| a.into()).collect(),
                mutable: cfg.mutable,
            })
        }
    }
}
pub mod error {
    use cosmwasm_std::StdError;
    use thiserror::Error;
    pub enum ContractError {
        #[error("{0}")]
        Std(#[from] StdError),
        #[error("Unauthorized")]
        Unauthorized {},
    }
    #[allow(unused_qualifications)]
    impl std::error::Error for ContractError {
        fn source(&self) -> std::option::Option<&(dyn std::error::Error + 'static)> {
            use thiserror::private::AsDynError;
            #[allow(deprecated)]
            match self {
                ContractError::Std { 0: source, .. } => {
                    std::option::Option::Some(source.as_dyn_error())
                }
                ContractError::Unauthorized { .. } => std::option::Option::None,
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::fmt::Display for ContractError {
        fn fmt(&self, __formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            #[allow(unused_imports)]
            use thiserror::private::{DisplayAsDisplay, PathAsDisplay};
            #[allow(unused_variables, deprecated, clippy::used_underscore_binding)]
            match self {
                ContractError::Std(_0) => __formatter.write_fmt(::core::fmt::Arguments::new_v1(
                    &[""],
                    &[::core::fmt::ArgumentV1::new_display(&_0.as_display())],
                )),
                ContractError::Unauthorized {} => {
                    __formatter.write_fmt(::core::fmt::Arguments::new_v1(&["Unauthorized"], &[]))
                }
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::convert::From<StdError> for ContractError {
        #[allow(deprecated)]
        fn from(source: StdError) -> Self {
            ContractError::Std { 0: source }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ContractError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&ContractError::Std(ref __self_0),) => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "Std");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&ContractError::Unauthorized {},) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "Unauthorized");
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for ContractError {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for ContractError {
        #[inline]
        fn eq(&self, other: &ContractError) -> bool {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (&ContractError::Std(ref __self_0), &ContractError::Std(ref __arg_1_0)) => {
                            (*__self_0) == (*__arg_1_0)
                        }
                        _ => true,
                    }
                } else {
                    false
                }
            }
        }
        #[inline]
        fn ne(&self, other: &ContractError) -> bool {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (&ContractError::Std(ref __self_0), &ContractError::Std(ref __arg_1_0)) => {
                            (*__self_0) != (*__arg_1_0)
                        }
                        _ => false,
                    }
                } else {
                    true
                }
            }
        }
    }
}
pub mod interfaces {
    use crate::msg::AdminListResponse;
    use cosmwasm_std::{CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response};
    use cw1::query::CanExecuteResponse;
    pub trait Cw1<T>
    where
        T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
    {
        type Error: From<StdError>;
        fn execute(
            &self,
            ctx: (DepsMut, Env, MessageInfo),
            msgs: Vec<CosmosMsg<T>>,
        ) -> Result<Response<T>, Self::Error>;
        fn can_execute(
            &self,
            ctx: (Deps, Env),
            sender: String,
            msg: CosmosMsg<T>,
        ) -> Result<CanExecuteResponse, Self::Error>;
    }
    pub mod cw1 {
        use super::*;
        #[serde(rename_all = "snake_case")]
        pub enum ExecMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            Execute { msgs: Vec<CosmosMsg<T>> },
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<T> _serde::Serialize for ExecMsg<T>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
                T: _serde::Serialize,
            {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    match *self {
                        ExecMsg::Execute { ref msgs } => {
                            let mut __serde_state =
                                match _serde::Serializer::serialize_struct_variant(
                                    __serializer,
                                    "ExecMsg",
                                    0u32,
                                    "execute",
                                    0 + 1,
                                ) {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                };
                            match _serde::ser::SerializeStructVariant::serialize_field(
                                &mut __serde_state,
                                "msgs",
                                msgs,
                            ) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            };
                            _serde::ser::SerializeStructVariant::end(__serde_state)
                        }
                    }
                }
            }
        };
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<'de, T> _serde::Deserialize<'de> for ExecMsg<T>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
                T: _serde::Deserialize<'de>,
            {
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    #[allow(non_camel_case_types)]
                    enum __Field {
                        __field0,
                    }
                    struct __FieldVisitor;
                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "variant identifier",
                            )
                        }
                        fn visit_u64<__E>(
                            self,
                            __value: u64,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                0u64 => _serde::__private::Ok(__Field::__field0),
                                _ => _serde::__private::Err(_serde::de::Error::invalid_value(
                                    _serde::de::Unexpected::Unsigned(__value),
                                    &"variant index 0 <= i < 1",
                                )),
                            }
                        }
                        fn visit_str<__E>(
                            self,
                            __value: &str,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                "execute" => _serde::__private::Ok(__Field::__field0),
                                _ => _serde::__private::Err(_serde::de::Error::unknown_variant(
                                    __value, VARIANTS,
                                )),
                            }
                        }
                        fn visit_bytes<__E>(
                            self,
                            __value: &[u8],
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                b"execute" => _serde::__private::Ok(__Field::__field0),
                                _ => {
                                    let __value = &_serde::__private::from_utf8_lossy(__value);
                                    _serde::__private::Err(_serde::de::Error::unknown_variant(
                                        __value, VARIANTS,
                                    ))
                                }
                            }
                        }
                    }
                    impl<'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(
                            __deserializer: __D,
                        ) -> _serde::__private::Result<Self, __D::Error>
                        where
                            __D: _serde::Deserializer<'de>,
                        {
                            _serde::Deserializer::deserialize_identifier(
                                __deserializer,
                                __FieldVisitor,
                            )
                        }
                    }
                    struct __Visitor<'de, T>
                    where
                        T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
                        T: _serde::Deserialize<'de>,
                    {
                        marker: _serde::__private::PhantomData<ExecMsg<T>>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    impl<'de, T> _serde::de::Visitor<'de> for __Visitor<'de, T>
                    where
                        T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
                        T: _serde::Deserialize<'de>,
                    {
                        type Value = ExecMsg<T>;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter, "enum ExecMsg")
                        }
                        fn visit_enum<__A>(
                            self,
                            __data: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::EnumAccess<'de>,
                        {
                            match match _serde::de::EnumAccess::variant(__data) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            } {
                                (__Field::__field0, __variant) => {
                                    #[allow(non_camel_case_types)]
                                    enum __Field {
                                        __field0,
                                        __ignore,
                                    }
                                    struct __FieldVisitor;
                                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                                        type Value = __Field;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "field identifier",
                                            )
                                        }
                                        fn visit_u64<__E>(
                                            self,
                                            __value: u64,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                0u64 => _serde::__private::Ok(__Field::__field0),
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_str<__E>(
                                            self,
                                            __value: &str,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                "msgs" => _serde::__private::Ok(__Field::__field0),
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_bytes<__E>(
                                            self,
                                            __value: &[u8],
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                b"msgs" => _serde::__private::Ok(__Field::__field0),
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                    }
                                    impl<'de> _serde::Deserialize<'de> for __Field {
                                        #[inline]
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::Deserializer::deserialize_identifier(
                                                __deserializer,
                                                __FieldVisitor,
                                            )
                                        }
                                    }
                                    struct __Visitor<'de, T>
                                    where
                                        T: std::fmt::Debug
                                            + PartialEq
                                            + Clone
                                            + schemars::JsonSchema,
                                        T: _serde::Deserialize<'de>,
                                    {
                                        marker: _serde::__private::PhantomData<ExecMsg<T>>,
                                        lifetime: _serde::__private::PhantomData<&'de ()>,
                                    }
                                    impl<'de, T> _serde::de::Visitor<'de> for __Visitor<'de, T>
                                    where
                                        T: std::fmt::Debug
                                            + PartialEq
                                            + Clone
                                            + schemars::JsonSchema,
                                        T: _serde::Deserialize<'de>,
                                    {
                                        type Value = ExecMsg<T>;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "struct variant ExecMsg::Execute",
                                            )
                                        }
                                        #[inline]
                                        fn visit_seq<__A>(
                                            self,
                                            mut __seq: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::SeqAccess<'de>,
                                        {
                                            let __field0 =
                                                match match _serde::de::SeqAccess::next_element::<
                                                    Vec<CosmosMsg<T>>,
                                                >(
                                                    &mut __seq
                                                ) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                } {
                                                    _serde::__private::Some(__value) => __value,
                                                    _serde::__private::None => {
                                                        return _serde :: __private :: Err (_serde :: de :: Error :: invalid_length (0usize , & "struct variant ExecMsg::Execute with 1 element")) ;
                                                    }
                                                };
                                            _serde::__private::Ok(ExecMsg::Execute {
                                                msgs: __field0,
                                            })
                                        }
                                        #[inline]
                                        fn visit_map<__A>(
                                            self,
                                            mut __map: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::MapAccess<'de>,
                                        {
                                            let mut __field0: _serde::__private::Option<
                                                Vec<CosmosMsg<T>>,
                                            > = _serde::__private::None;
                                            while let _serde::__private::Some(__key) =
                                                match _serde::de::MapAccess::next_key::<__Field>(
                                                    &mut __map,
                                                ) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                }
                                            {
                                                match __key {
                                                    __Field::__field0 => {
                                                        if _serde::__private::Option::is_some(
                                                            &__field0,
                                                        ) {
                                                            return _serde :: __private :: Err (< __A :: Error as _serde :: de :: Error > :: duplicate_field ("msgs")) ;
                                                        }
                                                        __field0 = _serde::__private::Some(
                                                            match _serde::de::MapAccess::next_value::<
                                                                Vec<CosmosMsg<T>>,
                                                            >(
                                                                &mut __map
                                                            ) {
                                                                _serde::__private::Ok(__val) => {
                                                                    __val
                                                                }
                                                                _serde::__private::Err(__err) => {
                                                                    return _serde::__private::Err(
                                                                        __err,
                                                                    );
                                                                }
                                                            },
                                                        );
                                                    }
                                                    _ => {
                                                        let _ =
                                                            match _serde::de::MapAccess::next_value::<
                                                                _serde::de::IgnoredAny,
                                                            >(
                                                                &mut __map
                                                            ) {
                                                                _serde::__private::Ok(__val) => {
                                                                    __val
                                                                }
                                                                _serde::__private::Err(__err) => {
                                                                    return _serde::__private::Err(
                                                                        __err,
                                                                    );
                                                                }
                                                            };
                                                    }
                                                }
                                            }
                                            let __field0 = match __field0 {
                                                _serde::__private::Some(__field0) => __field0,
                                                _serde::__private::None => {
                                                    match _serde::__private::de::missing_field(
                                                        "msgs",
                                                    ) {
                                                        _serde::__private::Ok(__val) => __val,
                                                        _serde::__private::Err(__err) => {
                                                            return _serde::__private::Err(__err);
                                                        }
                                                    }
                                                }
                                            };
                                            _serde::__private::Ok(ExecMsg::Execute {
                                                msgs: __field0,
                                            })
                                        }
                                    }
                                    const FIELDS: &'static [&'static str] = &["msgs"];
                                    _serde::de::VariantAccess::struct_variant(
                                        __variant,
                                        FIELDS,
                                        __Visitor {
                                            marker: _serde::__private::PhantomData::<ExecMsg<T>>,
                                            lifetime: _serde::__private::PhantomData,
                                        },
                                    )
                                }
                            }
                        }
                    }
                    const VARIANTS: &'static [&'static str] = &["execute"];
                    _serde::Deserializer::deserialize_enum(
                        __deserializer,
                        "ExecMsg",
                        VARIANTS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<ExecMsg<T>>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
            }
        };
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl<T: ::core::clone::Clone> ::core::clone::Clone for ExecMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            #[inline]
            fn clone(&self) -> ExecMsg<T> {
                match (&*self,) {
                    (&ExecMsg::Execute { msgs: ref __self_0 },) => ExecMsg::Execute {
                        msgs: ::core::clone::Clone::clone(&(*__self_0)),
                    },
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl<T: ::core::fmt::Debug> ::core::fmt::Debug for ExecMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match (&*self,) {
                    (&ExecMsg::Execute { msgs: ref __self_0 },) => {
                        let debug_trait_builder =
                            &mut ::core::fmt::Formatter::debug_struct(f, "Execute");
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "msgs",
                            &&(*__self_0),
                        );
                        ::core::fmt::DebugStruct::finish(debug_trait_builder)
                    }
                }
            }
        }
        impl<T> ::core::marker::StructuralPartialEq for ExecMsg<T> where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema
        {
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl<T: ::core::cmp::PartialEq> ::core::cmp::PartialEq for ExecMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            #[inline]
            fn eq(&self, other: &ExecMsg<T>) -> bool {
                match (&*self, &*other) {
                    (
                        &ExecMsg::Execute { msgs: ref __self_0 },
                        &ExecMsg::Execute {
                            msgs: ref __arg_1_0,
                        },
                    ) => (*__self_0) == (*__arg_1_0),
                }
            }
            #[inline]
            fn ne(&self, other: &ExecMsg<T>) -> bool {
                match (&*self, &*other) {
                    (
                        &ExecMsg::Execute { msgs: ref __self_0 },
                        &ExecMsg::Execute {
                            msgs: ref __arg_1_0,
                        },
                    ) => (*__self_0) != (*__arg_1_0),
                }
            }
        }
        const _: () = {
            #[automatically_derived]
            #[allow(unused_braces)]
            impl<T: schemars::JsonSchema> schemars::JsonSchema for ExecMsg<T>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
            {
                fn schema_name() -> std::string::String {
                    {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["ExecMsg_for_"],
                            &[::core::fmt::ArgumentV1::new_display(&T::schema_name())],
                        ));
                        res
                    }
                }
                fn json_schema(
                    gen: &mut schemars::gen::SchemaGenerator,
                ) -> schemars::schema::Schema {
                    schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                        subschemas: Some(Box::new(schemars::schema::SubschemaValidation {
                            one_of: Some(<[_]>::into_vec(box [schemars::schema::Schema::Object(
                                schemars::schema::SchemaObject {
                                    instance_type: Some(
                                        schemars::schema::InstanceType::Object.into(),
                                    ),
                                    object: Some(Box::new(schemars::schema::ObjectValidation {
                                        properties: {
                                            let mut props = schemars::Map::new();
                                            props . insert ("execute" . to_owned () , { let mut schema_object = schemars :: schema :: SchemaObject { instance_type : Some (schemars :: schema :: InstanceType :: Object . into ()) , .. Default :: default () } ; let object_validation = schema_object . object () ; { object_validation . properties . insert ("msgs" . to_owned () , gen . subschema_for :: < Vec < CosmosMsg < T > > > ()) ; if ! < Vec < CosmosMsg < T > > as schemars :: JsonSchema > :: _schemars_private_is_option () { object_validation . required . insert ("msgs" . to_owned ()) ; } } schemars :: schema :: Schema :: Object (schema_object) }) ;
                                            props
                                        },
                                        required: {
                                            let mut required = schemars::Set::new();
                                            required.insert("execute".to_owned());
                                            required
                                        },
                                        additional_properties: Some(Box::new(false.into())),
                                        ..Default::default()
                                    })),
                                    ..Default::default()
                                },
                            )])),
                            ..Default::default()
                        })),
                        ..Default::default()
                    })
                }
            };
        };
        impl<T> ExecMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            pub fn dispatch<C: Cw1<T>>(
                self,
                contract: &C,
                ctx: (
                    cosmwasm_std::DepsMut,
                    cosmwasm_std::Env,
                    cosmwasm_std::MessageInfo,
                ),
            ) -> std::result::Result<cosmwasm_std::Response, C::Error>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
            {
                use ExecMsg::*;
                match self {
                    Execute { msgs } => contract.execute(ctx.into(), msgs).map_err(Into::into),
                }
            }
        }
        #[serde(rename_all = "snake_case")]
        pub enum QueryMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            CanExecute { sender: String, msg: CosmosMsg<T> },
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<T> _serde::Serialize for QueryMsg<T>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
                T: _serde::Serialize,
            {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    match *self {
                        QueryMsg::CanExecute {
                            ref sender,
                            ref msg,
                        } => {
                            let mut __serde_state =
                                match _serde::Serializer::serialize_struct_variant(
                                    __serializer,
                                    "QueryMsg",
                                    0u32,
                                    "can_execute",
                                    0 + 1 + 1,
                                ) {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                };
                            match _serde::ser::SerializeStructVariant::serialize_field(
                                &mut __serde_state,
                                "sender",
                                sender,
                            ) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            };
                            match _serde::ser::SerializeStructVariant::serialize_field(
                                &mut __serde_state,
                                "msg",
                                msg,
                            ) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            };
                            _serde::ser::SerializeStructVariant::end(__serde_state)
                        }
                    }
                }
            }
        };
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<'de, T> _serde::Deserialize<'de> for QueryMsg<T>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
                T: _serde::Deserialize<'de>,
            {
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    #[allow(non_camel_case_types)]
                    enum __Field {
                        __field0,
                    }
                    struct __FieldVisitor;
                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "variant identifier",
                            )
                        }
                        fn visit_u64<__E>(
                            self,
                            __value: u64,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                0u64 => _serde::__private::Ok(__Field::__field0),
                                _ => _serde::__private::Err(_serde::de::Error::invalid_value(
                                    _serde::de::Unexpected::Unsigned(__value),
                                    &"variant index 0 <= i < 1",
                                )),
                            }
                        }
                        fn visit_str<__E>(
                            self,
                            __value: &str,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                "can_execute" => _serde::__private::Ok(__Field::__field0),
                                _ => _serde::__private::Err(_serde::de::Error::unknown_variant(
                                    __value, VARIANTS,
                                )),
                            }
                        }
                        fn visit_bytes<__E>(
                            self,
                            __value: &[u8],
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                b"can_execute" => _serde::__private::Ok(__Field::__field0),
                                _ => {
                                    let __value = &_serde::__private::from_utf8_lossy(__value);
                                    _serde::__private::Err(_serde::de::Error::unknown_variant(
                                        __value, VARIANTS,
                                    ))
                                }
                            }
                        }
                    }
                    impl<'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(
                            __deserializer: __D,
                        ) -> _serde::__private::Result<Self, __D::Error>
                        where
                            __D: _serde::Deserializer<'de>,
                        {
                            _serde::Deserializer::deserialize_identifier(
                                __deserializer,
                                __FieldVisitor,
                            )
                        }
                    }
                    struct __Visitor<'de, T>
                    where
                        T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
                        T: _serde::Deserialize<'de>,
                    {
                        marker: _serde::__private::PhantomData<QueryMsg<T>>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    impl<'de, T> _serde::de::Visitor<'de> for __Visitor<'de, T>
                    where
                        T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
                        T: _serde::Deserialize<'de>,
                    {
                        type Value = QueryMsg<T>;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter, "enum QueryMsg")
                        }
                        fn visit_enum<__A>(
                            self,
                            __data: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::EnumAccess<'de>,
                        {
                            match match _serde::de::EnumAccess::variant(__data) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            } {
                                (__Field::__field0, __variant) => {
                                    #[allow(non_camel_case_types)]
                                    enum __Field {
                                        __field0,
                                        __field1,
                                        __ignore,
                                    }
                                    struct __FieldVisitor;
                                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                                        type Value = __Field;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "field identifier",
                                            )
                                        }
                                        fn visit_u64<__E>(
                                            self,
                                            __value: u64,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                0u64 => _serde::__private::Ok(__Field::__field0),
                                                1u64 => _serde::__private::Ok(__Field::__field1),
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_str<__E>(
                                            self,
                                            __value: &str,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                "sender" => {
                                                    _serde::__private::Ok(__Field::__field0)
                                                }
                                                "msg" => _serde::__private::Ok(__Field::__field1),
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_bytes<__E>(
                                            self,
                                            __value: &[u8],
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                b"sender" => {
                                                    _serde::__private::Ok(__Field::__field0)
                                                }
                                                b"msg" => _serde::__private::Ok(__Field::__field1),
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                    }
                                    impl<'de> _serde::Deserialize<'de> for __Field {
                                        #[inline]
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::Deserializer::deserialize_identifier(
                                                __deserializer,
                                                __FieldVisitor,
                                            )
                                        }
                                    }
                                    struct __Visitor<'de, T>
                                    where
                                        T: std::fmt::Debug
                                            + PartialEq
                                            + Clone
                                            + schemars::JsonSchema,
                                        T: _serde::Deserialize<'de>,
                                    {
                                        marker: _serde::__private::PhantomData<QueryMsg<T>>,
                                        lifetime: _serde::__private::PhantomData<&'de ()>,
                                    }
                                    impl<'de, T> _serde::de::Visitor<'de> for __Visitor<'de, T>
                                    where
                                        T: std::fmt::Debug
                                            + PartialEq
                                            + Clone
                                            + schemars::JsonSchema,
                                        T: _serde::Deserialize<'de>,
                                    {
                                        type Value = QueryMsg<T>;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "struct variant QueryMsg::CanExecute",
                                            )
                                        }
                                        #[inline]
                                        fn visit_seq<__A>(
                                            self,
                                            mut __seq: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::SeqAccess<'de>,
                                        {
                                            let __field0 =
                                                match match _serde::de::SeqAccess::next_element::<
                                                    String,
                                                >(
                                                    &mut __seq
                                                ) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                } {
                                                    _serde::__private::Some(__value) => __value,
                                                    _serde::__private::None => {
                                                        return _serde :: __private :: Err (_serde :: de :: Error :: invalid_length (0usize , & "struct variant QueryMsg::CanExecute with 2 elements")) ;
                                                    }
                                                };
                                            let __field1 =
                                                match match _serde::de::SeqAccess::next_element::<
                                                    CosmosMsg<T>,
                                                >(
                                                    &mut __seq
                                                ) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                } {
                                                    _serde::__private::Some(__value) => __value,
                                                    _serde::__private::None => {
                                                        return _serde :: __private :: Err (_serde :: de :: Error :: invalid_length (1usize , & "struct variant QueryMsg::CanExecute with 2 elements")) ;
                                                    }
                                                };
                                            _serde::__private::Ok(QueryMsg::CanExecute {
                                                sender: __field0,
                                                msg: __field1,
                                            })
                                        }
                                        #[inline]
                                        fn visit_map<__A>(
                                            self,
                                            mut __map: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::MapAccess<'de>,
                                        {
                                            let mut __field0: _serde::__private::Option<String> =
                                                _serde::__private::None;
                                            let mut __field1: _serde::__private::Option<
                                                CosmosMsg<T>,
                                            > = _serde::__private::None;
                                            while let _serde::__private::Some(__key) =
                                                match _serde::de::MapAccess::next_key::<__Field>(
                                                    &mut __map,
                                                ) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                }
                                            {
                                                match __key {
                                                    __Field::__field0 => {
                                                        if _serde::__private::Option::is_some(
                                                            &__field0,
                                                        ) {
                                                            return _serde :: __private :: Err (< __A :: Error as _serde :: de :: Error > :: duplicate_field ("sender")) ;
                                                        }
                                                        __field0 = _serde::__private::Some(
                                                            match _serde::de::MapAccess::next_value::<
                                                                String,
                                                            >(
                                                                &mut __map
                                                            ) {
                                                                _serde::__private::Ok(__val) => {
                                                                    __val
                                                                }
                                                                _serde::__private::Err(__err) => {
                                                                    return _serde::__private::Err(
                                                                        __err,
                                                                    );
                                                                }
                                                            },
                                                        );
                                                    }
                                                    __Field::__field1 => {
                                                        if _serde::__private::Option::is_some(
                                                            &__field1,
                                                        ) {
                                                            return _serde :: __private :: Err (< __A :: Error as _serde :: de :: Error > :: duplicate_field ("msg")) ;
                                                        }
                                                        __field1 = _serde::__private::Some(
                                                            match _serde::de::MapAccess::next_value::<
                                                                CosmosMsg<T>,
                                                            >(
                                                                &mut __map
                                                            ) {
                                                                _serde::__private::Ok(__val) => {
                                                                    __val
                                                                }
                                                                _serde::__private::Err(__err) => {
                                                                    return _serde::__private::Err(
                                                                        __err,
                                                                    );
                                                                }
                                                            },
                                                        );
                                                    }
                                                    _ => {
                                                        let _ =
                                                            match _serde::de::MapAccess::next_value::<
                                                                _serde::de::IgnoredAny,
                                                            >(
                                                                &mut __map
                                                            ) {
                                                                _serde::__private::Ok(__val) => {
                                                                    __val
                                                                }
                                                                _serde::__private::Err(__err) => {
                                                                    return _serde::__private::Err(
                                                                        __err,
                                                                    );
                                                                }
                                                            };
                                                    }
                                                }
                                            }
                                            let __field0 = match __field0 {
                                                _serde::__private::Some(__field0) => __field0,
                                                _serde::__private::None => {
                                                    match _serde::__private::de::missing_field(
                                                        "sender",
                                                    ) {
                                                        _serde::__private::Ok(__val) => __val,
                                                        _serde::__private::Err(__err) => {
                                                            return _serde::__private::Err(__err);
                                                        }
                                                    }
                                                }
                                            };
                                            let __field1 = match __field1 {
                                                _serde::__private::Some(__field1) => __field1,
                                                _serde::__private::None => {
                                                    match _serde::__private::de::missing_field(
                                                        "msg",
                                                    ) {
                                                        _serde::__private::Ok(__val) => __val,
                                                        _serde::__private::Err(__err) => {
                                                            return _serde::__private::Err(__err);
                                                        }
                                                    }
                                                }
                                            };
                                            _serde::__private::Ok(QueryMsg::CanExecute {
                                                sender: __field0,
                                                msg: __field1,
                                            })
                                        }
                                    }
                                    const FIELDS: &'static [&'static str] = &["sender", "msg"];
                                    _serde::de::VariantAccess::struct_variant(
                                        __variant,
                                        FIELDS,
                                        __Visitor {
                                            marker: _serde::__private::PhantomData::<QueryMsg<T>>,
                                            lifetime: _serde::__private::PhantomData,
                                        },
                                    )
                                }
                            }
                        }
                    }
                    const VARIANTS: &'static [&'static str] = &["can_execute"];
                    _serde::Deserializer::deserialize_enum(
                        __deserializer,
                        "QueryMsg",
                        VARIANTS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<QueryMsg<T>>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
            }
        };
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl<T: ::core::clone::Clone> ::core::clone::Clone for QueryMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            #[inline]
            fn clone(&self) -> QueryMsg<T> {
                match (&*self,) {
                    (&QueryMsg::CanExecute {
                        sender: ref __self_0,
                        msg: ref __self_1,
                    },) => QueryMsg::CanExecute {
                        sender: ::core::clone::Clone::clone(&(*__self_0)),
                        msg: ::core::clone::Clone::clone(&(*__self_1)),
                    },
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl<T: ::core::fmt::Debug> ::core::fmt::Debug for QueryMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match (&*self,) {
                    (&QueryMsg::CanExecute {
                        sender: ref __self_0,
                        msg: ref __self_1,
                    },) => {
                        let debug_trait_builder =
                            &mut ::core::fmt::Formatter::debug_struct(f, "CanExecute");
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "sender",
                            &&(*__self_0),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "msg",
                            &&(*__self_1),
                        );
                        ::core::fmt::DebugStruct::finish(debug_trait_builder)
                    }
                }
            }
        }
        impl<T> ::core::marker::StructuralPartialEq for QueryMsg<T> where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema
        {
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl<T: ::core::cmp::PartialEq> ::core::cmp::PartialEq for QueryMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            #[inline]
            fn eq(&self, other: &QueryMsg<T>) -> bool {
                match (&*self, &*other) {
                    (
                        &QueryMsg::CanExecute {
                            sender: ref __self_0,
                            msg: ref __self_1,
                        },
                        &QueryMsg::CanExecute {
                            sender: ref __arg_1_0,
                            msg: ref __arg_1_1,
                        },
                    ) => (*__self_0) == (*__arg_1_0) && (*__self_1) == (*__arg_1_1),
                }
            }
            #[inline]
            fn ne(&self, other: &QueryMsg<T>) -> bool {
                match (&*self, &*other) {
                    (
                        &QueryMsg::CanExecute {
                            sender: ref __self_0,
                            msg: ref __self_1,
                        },
                        &QueryMsg::CanExecute {
                            sender: ref __arg_1_0,
                            msg: ref __arg_1_1,
                        },
                    ) => (*__self_0) != (*__arg_1_0) || (*__self_1) != (*__arg_1_1),
                }
            }
        }
        const _: () = {
            #[automatically_derived]
            #[allow(unused_braces)]
            impl<T: schemars::JsonSchema> schemars::JsonSchema for QueryMsg<T>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
            {
                fn schema_name() -> std::string::String {
                    {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["QueryMsg_for_"],
                            &[::core::fmt::ArgumentV1::new_display(&T::schema_name())],
                        ));
                        res
                    }
                }
                fn json_schema(
                    gen: &mut schemars::gen::SchemaGenerator,
                ) -> schemars::schema::Schema {
                    schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                        subschemas: Some(Box::new(schemars::schema::SubschemaValidation {
                            one_of: Some(<[_]>::into_vec(box [schemars::schema::Schema::Object(
                                schemars::schema::SchemaObject {
                                    instance_type: Some(
                                        schemars::schema::InstanceType::Object.into(),
                                    ),
                                    object: Some(Box::new(schemars::schema::ObjectValidation {
                                        properties: {
                                            let mut props = schemars::Map::new();
                                            props . insert ("can_execute" . to_owned () , { let mut schema_object = schemars :: schema :: SchemaObject { instance_type : Some (schemars :: schema :: InstanceType :: Object . into ()) , .. Default :: default () } ; let object_validation = schema_object . object () ; { object_validation . properties . insert ("sender" . to_owned () , gen . subschema_for :: < String > ()) ; if ! < String as schemars :: JsonSchema > :: _schemars_private_is_option () { object_validation . required . insert ("sender" . to_owned ()) ; } } { object_validation . properties . insert ("msg" . to_owned () , gen . subschema_for :: < CosmosMsg < T > > ()) ; if ! < CosmosMsg < T > as schemars :: JsonSchema > :: _schemars_private_is_option () { object_validation . required . insert ("msg" . to_owned ()) ; } } schemars :: schema :: Schema :: Object (schema_object) }) ;
                                            props
                                        },
                                        required: {
                                            let mut required = schemars::Set::new();
                                            required.insert("can_execute".to_owned());
                                            required
                                        },
                                        additional_properties: Some(Box::new(false.into())),
                                        ..Default::default()
                                    })),
                                    ..Default::default()
                                },
                            )])),
                            ..Default::default()
                        })),
                        ..Default::default()
                    })
                }
            };
        };
        impl<T> QueryMsg<T>
        where
            T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
        {
            pub fn dispatch<C: Cw1<T>>(
                self,
                contract: &C,
                ctx: (cosmwasm_std::Deps, cosmwasm_std::Env),
            ) -> std::result::Result<cosmwasm_std::Binary, C::Error>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
            {
                use QueryMsg::*;
                match self {
                    CanExecute { sender, msg } => {
                        cosmwasm_std::to_binary(&contract.can_execute(ctx.into(), sender, msg)?)
                            .map_err(Into::into)
                    }
                }
            }
        }
    }
    pub trait Whitelist<T>
    where
        T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
    {
        type Error: From<StdError>;
        fn freeze(&self, ctx: (DepsMut, Env, MessageInfo)) -> Result<Response<T>, Self::Error>;
        fn update_admins(
            &self,
            ctx: (DepsMut, Env, MessageInfo),
            admins: Vec<String>,
        ) -> Result<Response<T>, Self::Error>;
        fn admin_list(&self, ctx: (Deps, Env)) -> Result<AdminListResponse, Self::Error>;
    }
    pub mod whitelist {
        use super::*;
        #[serde(rename_all = "snake_case")]
        pub enum ExecMsg {
            Freeze {},
            UpdateAdmins { admins: Vec<String> },
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl _serde::Serialize for ExecMsg {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    match *self {
                        ExecMsg::Freeze {} => {
                            let __serde_state = match _serde::Serializer::serialize_struct_variant(
                                __serializer,
                                "ExecMsg",
                                0u32,
                                "freeze",
                                0,
                            ) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            };
                            _serde::ser::SerializeStructVariant::end(__serde_state)
                        }
                        ExecMsg::UpdateAdmins { ref admins } => {
                            let mut __serde_state =
                                match _serde::Serializer::serialize_struct_variant(
                                    __serializer,
                                    "ExecMsg",
                                    1u32,
                                    "update_admins",
                                    0 + 1,
                                ) {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                };
                            match _serde::ser::SerializeStructVariant::serialize_field(
                                &mut __serde_state,
                                "admins",
                                admins,
                            ) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            };
                            _serde::ser::SerializeStructVariant::end(__serde_state)
                        }
                    }
                }
            }
        };
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for ExecMsg {
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    #[allow(non_camel_case_types)]
                    enum __Field {
                        __field0,
                        __field1,
                    }
                    struct __FieldVisitor;
                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "variant identifier",
                            )
                        }
                        fn visit_u64<__E>(
                            self,
                            __value: u64,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                0u64 => _serde::__private::Ok(__Field::__field0),
                                1u64 => _serde::__private::Ok(__Field::__field1),
                                _ => _serde::__private::Err(_serde::de::Error::invalid_value(
                                    _serde::de::Unexpected::Unsigned(__value),
                                    &"variant index 0 <= i < 2",
                                )),
                            }
                        }
                        fn visit_str<__E>(
                            self,
                            __value: &str,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                "freeze" => _serde::__private::Ok(__Field::__field0),
                                "update_admins" => _serde::__private::Ok(__Field::__field1),
                                _ => _serde::__private::Err(_serde::de::Error::unknown_variant(
                                    __value, VARIANTS,
                                )),
                            }
                        }
                        fn visit_bytes<__E>(
                            self,
                            __value: &[u8],
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                b"freeze" => _serde::__private::Ok(__Field::__field0),
                                b"update_admins" => _serde::__private::Ok(__Field::__field1),
                                _ => {
                                    let __value = &_serde::__private::from_utf8_lossy(__value);
                                    _serde::__private::Err(_serde::de::Error::unknown_variant(
                                        __value, VARIANTS,
                                    ))
                                }
                            }
                        }
                    }
                    impl<'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(
                            __deserializer: __D,
                        ) -> _serde::__private::Result<Self, __D::Error>
                        where
                            __D: _serde::Deserializer<'de>,
                        {
                            _serde::Deserializer::deserialize_identifier(
                                __deserializer,
                                __FieldVisitor,
                            )
                        }
                    }
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<ExecMsg>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = ExecMsg;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter, "enum ExecMsg")
                        }
                        fn visit_enum<__A>(
                            self,
                            __data: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::EnumAccess<'de>,
                        {
                            match match _serde::de::EnumAccess::variant(__data) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            } {
                                (__Field::__field0, __variant) => {
                                    #[allow(non_camel_case_types)]
                                    enum __Field {
                                        __ignore,
                                    }
                                    struct __FieldVisitor;
                                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                                        type Value = __Field;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "field identifier",
                                            )
                                        }
                                        fn visit_u64<__E>(
                                            self,
                                            __value: u64,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_str<__E>(
                                            self,
                                            __value: &str,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_bytes<__E>(
                                            self,
                                            __value: &[u8],
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                    }
                                    impl<'de> _serde::Deserialize<'de> for __Field {
                                        #[inline]
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::Deserializer::deserialize_identifier(
                                                __deserializer,
                                                __FieldVisitor,
                                            )
                                        }
                                    }
                                    struct __Visitor<'de> {
                                        marker: _serde::__private::PhantomData<ExecMsg>,
                                        lifetime: _serde::__private::PhantomData<&'de ()>,
                                    }
                                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                                        type Value = ExecMsg;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "struct variant ExecMsg::Freeze",
                                            )
                                        }
                                        #[inline]
                                        fn visit_seq<__A>(
                                            self,
                                            _: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::SeqAccess<'de>,
                                        {
                                            _serde::__private::Ok(ExecMsg::Freeze {})
                                        }
                                        #[inline]
                                        fn visit_map<__A>(
                                            self,
                                            mut __map: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::MapAccess<'de>,
                                        {
                                            while let _serde::__private::Some(__key) =
                                                match _serde::de::MapAccess::next_key::<__Field>(
                                                    &mut __map,
                                                ) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                }
                                            {
                                                match __key {
                                                    _ => {
                                                        let _ =
                                                            match _serde::de::MapAccess::next_value::<
                                                                _serde::de::IgnoredAny,
                                                            >(
                                                                &mut __map
                                                            ) {
                                                                _serde::__private::Ok(__val) => {
                                                                    __val
                                                                }
                                                                _serde::__private::Err(__err) => {
                                                                    return _serde::__private::Err(
                                                                        __err,
                                                                    );
                                                                }
                                                            };
                                                    }
                                                }
                                            }
                                            _serde::__private::Ok(ExecMsg::Freeze {})
                                        }
                                    }
                                    const FIELDS: &'static [&'static str] = &[];
                                    _serde::de::VariantAccess::struct_variant(
                                        __variant,
                                        FIELDS,
                                        __Visitor {
                                            marker: _serde::__private::PhantomData::<ExecMsg>,
                                            lifetime: _serde::__private::PhantomData,
                                        },
                                    )
                                }
                                (__Field::__field1, __variant) => {
                                    #[allow(non_camel_case_types)]
                                    enum __Field {
                                        __field0,
                                        __ignore,
                                    }
                                    struct __FieldVisitor;
                                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                                        type Value = __Field;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "field identifier",
                                            )
                                        }
                                        fn visit_u64<__E>(
                                            self,
                                            __value: u64,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                0u64 => _serde::__private::Ok(__Field::__field0),
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_str<__E>(
                                            self,
                                            __value: &str,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                "admins" => {
                                                    _serde::__private::Ok(__Field::__field0)
                                                }
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_bytes<__E>(
                                            self,
                                            __value: &[u8],
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                b"admins" => {
                                                    _serde::__private::Ok(__Field::__field0)
                                                }
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                    }
                                    impl<'de> _serde::Deserialize<'de> for __Field {
                                        #[inline]
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::Deserializer::deserialize_identifier(
                                                __deserializer,
                                                __FieldVisitor,
                                            )
                                        }
                                    }
                                    struct __Visitor<'de> {
                                        marker: _serde::__private::PhantomData<ExecMsg>,
                                        lifetime: _serde::__private::PhantomData<&'de ()>,
                                    }
                                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                                        type Value = ExecMsg;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "struct variant ExecMsg::UpdateAdmins",
                                            )
                                        }
                                        #[inline]
                                        fn visit_seq<__A>(
                                            self,
                                            mut __seq: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::SeqAccess<'de>,
                                        {
                                            let __field0 =
                                                match match _serde::de::SeqAccess::next_element::<
                                                    Vec<String>,
                                                >(
                                                    &mut __seq
                                                ) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                } {
                                                    _serde::__private::Some(__value) => __value,
                                                    _serde::__private::None => {
                                                        return _serde :: __private :: Err (_serde :: de :: Error :: invalid_length (0usize , & "struct variant ExecMsg::UpdateAdmins with 1 element")) ;
                                                    }
                                                };
                                            _serde::__private::Ok(ExecMsg::UpdateAdmins {
                                                admins: __field0,
                                            })
                                        }
                                        #[inline]
                                        fn visit_map<__A>(
                                            self,
                                            mut __map: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::MapAccess<'de>,
                                        {
                                            let mut __field0: _serde::__private::Option<
                                                Vec<String>,
                                            > = _serde::__private::None;
                                            while let _serde::__private::Some(__key) =
                                                match _serde::de::MapAccess::next_key::<__Field>(
                                                    &mut __map,
                                                ) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                }
                                            {
                                                match __key {
                                                    __Field::__field0 => {
                                                        if _serde::__private::Option::is_some(
                                                            &__field0,
                                                        ) {
                                                            return _serde :: __private :: Err (< __A :: Error as _serde :: de :: Error > :: duplicate_field ("admins")) ;
                                                        }
                                                        __field0 = _serde::__private::Some(
                                                            match _serde::de::MapAccess::next_value::<
                                                                Vec<String>,
                                                            >(
                                                                &mut __map
                                                            ) {
                                                                _serde::__private::Ok(__val) => {
                                                                    __val
                                                                }
                                                                _serde::__private::Err(__err) => {
                                                                    return _serde::__private::Err(
                                                                        __err,
                                                                    );
                                                                }
                                                            },
                                                        );
                                                    }
                                                    _ => {
                                                        let _ =
                                                            match _serde::de::MapAccess::next_value::<
                                                                _serde::de::IgnoredAny,
                                                            >(
                                                                &mut __map
                                                            ) {
                                                                _serde::__private::Ok(__val) => {
                                                                    __val
                                                                }
                                                                _serde::__private::Err(__err) => {
                                                                    return _serde::__private::Err(
                                                                        __err,
                                                                    );
                                                                }
                                                            };
                                                    }
                                                }
                                            }
                                            let __field0 = match __field0 {
                                                _serde::__private::Some(__field0) => __field0,
                                                _serde::__private::None => {
                                                    match _serde::__private::de::missing_field(
                                                        "admins",
                                                    ) {
                                                        _serde::__private::Ok(__val) => __val,
                                                        _serde::__private::Err(__err) => {
                                                            return _serde::__private::Err(__err);
                                                        }
                                                    }
                                                }
                                            };
                                            _serde::__private::Ok(ExecMsg::UpdateAdmins {
                                                admins: __field0,
                                            })
                                        }
                                    }
                                    const FIELDS: &'static [&'static str] = &["admins"];
                                    _serde::de::VariantAccess::struct_variant(
                                        __variant,
                                        FIELDS,
                                        __Visitor {
                                            marker: _serde::__private::PhantomData::<ExecMsg>,
                                            lifetime: _serde::__private::PhantomData,
                                        },
                                    )
                                }
                            }
                        }
                    }
                    const VARIANTS: &'static [&'static str] = &["freeze", "update_admins"];
                    _serde::Deserializer::deserialize_enum(
                        __deserializer,
                        "ExecMsg",
                        VARIANTS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<ExecMsg>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
            }
        };
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for ExecMsg {
            #[inline]
            fn clone(&self) -> ExecMsg {
                match (&*self,) {
                    (&ExecMsg::Freeze {},) => ExecMsg::Freeze {},
                    (&ExecMsg::UpdateAdmins {
                        admins: ref __self_0,
                    },) => ExecMsg::UpdateAdmins {
                        admins: ::core::clone::Clone::clone(&(*__self_0)),
                    },
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for ExecMsg {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match (&*self,) {
                    (&ExecMsg::Freeze {},) => {
                        let debug_trait_builder =
                            &mut ::core::fmt::Formatter::debug_struct(f, "Freeze");
                        ::core::fmt::DebugStruct::finish(debug_trait_builder)
                    }
                    (&ExecMsg::UpdateAdmins {
                        admins: ref __self_0,
                    },) => {
                        let debug_trait_builder =
                            &mut ::core::fmt::Formatter::debug_struct(f, "UpdateAdmins");
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "admins",
                            &&(*__self_0),
                        );
                        ::core::fmt::DebugStruct::finish(debug_trait_builder)
                    }
                }
            }
        }
        impl ::core::marker::StructuralPartialEq for ExecMsg {}
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for ExecMsg {
            #[inline]
            fn eq(&self, other: &ExecMsg) -> bool {
                {
                    let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                    let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) {
                            (
                                &ExecMsg::UpdateAdmins {
                                    admins: ref __self_0,
                                },
                                &ExecMsg::UpdateAdmins {
                                    admins: ref __arg_1_0,
                                },
                            ) => (*__self_0) == (*__arg_1_0),
                            _ => true,
                        }
                    } else {
                        false
                    }
                }
            }
            #[inline]
            fn ne(&self, other: &ExecMsg) -> bool {
                {
                    let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                    let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) {
                            (
                                &ExecMsg::UpdateAdmins {
                                    admins: ref __self_0,
                                },
                                &ExecMsg::UpdateAdmins {
                                    admins: ref __arg_1_0,
                                },
                            ) => (*__self_0) != (*__arg_1_0),
                            _ => false,
                        }
                    } else {
                        true
                    }
                }
            }
        }
        const _: () = {
            #[automatically_derived]
            #[allow(unused_braces)]
            impl schemars::JsonSchema for ExecMsg {
                fn schema_name() -> std::string::String {
                    "ExecMsg".to_owned()
                }
                fn json_schema(
                    gen: &mut schemars::gen::SchemaGenerator,
                ) -> schemars::schema::Schema {
                    schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                        subschemas: Some(Box::new(schemars::schema::SubschemaValidation {
                            one_of: Some(<[_]>::into_vec(box [
                                schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                                    instance_type: Some(
                                        schemars::schema::InstanceType::Object.into(),
                                    ),
                                    object: Some(Box::new(schemars::schema::ObjectValidation {
                                        properties: {
                                            let mut props = schemars::Map::new();
                                            props.insert("freeze".to_owned(), {
                                                let mut schema_object =
                                                    schemars::schema::SchemaObject {
                                                        instance_type: Some(
                                                            schemars::schema::InstanceType::Object
                                                                .into(),
                                                        ),
                                                        ..Default::default()
                                                    };
                                                let object_validation = schema_object.object();
                                                schemars::schema::Schema::Object(schema_object)
                                            });
                                            props
                                        },
                                        required: {
                                            let mut required = schemars::Set::new();
                                            required.insert("freeze".to_owned());
                                            required
                                        },
                                        additional_properties: Some(Box::new(false.into())),
                                        ..Default::default()
                                    })),
                                    ..Default::default()
                                }),
                                schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                                    instance_type: Some(
                                        schemars::schema::InstanceType::Object.into(),
                                    ),
                                    object: Some(Box::new(schemars::schema::ObjectValidation {
                                        properties: {
                                            let mut props = schemars::Map::new();
                                            props . insert ("update_admins" . to_owned () , { let mut schema_object = schemars :: schema :: SchemaObject { instance_type : Some (schemars :: schema :: InstanceType :: Object . into ()) , .. Default :: default () } ; let object_validation = schema_object . object () ; { object_validation . properties . insert ("admins" . to_owned () , gen . subschema_for :: < Vec < String > > ()) ; if ! < Vec < String > as schemars :: JsonSchema > :: _schemars_private_is_option () { object_validation . required . insert ("admins" . to_owned ()) ; } } schemars :: schema :: Schema :: Object (schema_object) }) ;
                                            props
                                        },
                                        required: {
                                            let mut required = schemars::Set::new();
                                            required.insert("update_admins".to_owned());
                                            required
                                        },
                                        additional_properties: Some(Box::new(false.into())),
                                        ..Default::default()
                                    })),
                                    ..Default::default()
                                }),
                            ])),
                            ..Default::default()
                        })),
                        ..Default::default()
                    })
                }
            };
        };
        impl ExecMsg {
            pub fn dispatch<C: Whitelist<T>, T>(
                self,
                contract: &C,
                ctx: (
                    cosmwasm_std::DepsMut,
                    cosmwasm_std::Env,
                    cosmwasm_std::MessageInfo,
                ),
            ) -> std::result::Result<cosmwasm_std::Response, C::Error>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
            {
                use ExecMsg::*;
                match self {
                    Freeze {} => contract.freeze(ctx.into()).map_err(Into::into),
                    UpdateAdmins { admins } => contract
                        .update_admins(ctx.into(), admins)
                        .map_err(Into::into),
                }
            }
        }
        #[serde(rename_all = "snake_case")]
        pub enum QueryMsg {
            AdminList {},
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl _serde::Serialize for QueryMsg {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    match *self {
                        QueryMsg::AdminList {} => {
                            let __serde_state = match _serde::Serializer::serialize_struct_variant(
                                __serializer,
                                "QueryMsg",
                                0u32,
                                "admin_list",
                                0,
                            ) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            };
                            _serde::ser::SerializeStructVariant::end(__serde_state)
                        }
                    }
                }
            }
        };
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for QueryMsg {
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    #[allow(non_camel_case_types)]
                    enum __Field {
                        __field0,
                    }
                    struct __FieldVisitor;
                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "variant identifier",
                            )
                        }
                        fn visit_u64<__E>(
                            self,
                            __value: u64,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                0u64 => _serde::__private::Ok(__Field::__field0),
                                _ => _serde::__private::Err(_serde::de::Error::invalid_value(
                                    _serde::de::Unexpected::Unsigned(__value),
                                    &"variant index 0 <= i < 1",
                                )),
                            }
                        }
                        fn visit_str<__E>(
                            self,
                            __value: &str,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                "admin_list" => _serde::__private::Ok(__Field::__field0),
                                _ => _serde::__private::Err(_serde::de::Error::unknown_variant(
                                    __value, VARIANTS,
                                )),
                            }
                        }
                        fn visit_bytes<__E>(
                            self,
                            __value: &[u8],
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                b"admin_list" => _serde::__private::Ok(__Field::__field0),
                                _ => {
                                    let __value = &_serde::__private::from_utf8_lossy(__value);
                                    _serde::__private::Err(_serde::de::Error::unknown_variant(
                                        __value, VARIANTS,
                                    ))
                                }
                            }
                        }
                    }
                    impl<'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(
                            __deserializer: __D,
                        ) -> _serde::__private::Result<Self, __D::Error>
                        where
                            __D: _serde::Deserializer<'de>,
                        {
                            _serde::Deserializer::deserialize_identifier(
                                __deserializer,
                                __FieldVisitor,
                            )
                        }
                    }
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<QueryMsg>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = QueryMsg;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter, "enum QueryMsg")
                        }
                        fn visit_enum<__A>(
                            self,
                            __data: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::EnumAccess<'de>,
                        {
                            match match _serde::de::EnumAccess::variant(__data) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            } {
                                (__Field::__field0, __variant) => {
                                    #[allow(non_camel_case_types)]
                                    enum __Field {
                                        __ignore,
                                    }
                                    struct __FieldVisitor;
                                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                                        type Value = __Field;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "field identifier",
                                            )
                                        }
                                        fn visit_u64<__E>(
                                            self,
                                            __value: u64,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_str<__E>(
                                            self,
                                            __value: &str,
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                        fn visit_bytes<__E>(
                                            self,
                                            __value: &[u8],
                                        ) -> _serde::__private::Result<Self::Value, __E>
                                        where
                                            __E: _serde::de::Error,
                                        {
                                            match __value {
                                                _ => _serde::__private::Ok(__Field::__ignore),
                                            }
                                        }
                                    }
                                    impl<'de> _serde::Deserialize<'de> for __Field {
                                        #[inline]
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::Deserializer::deserialize_identifier(
                                                __deserializer,
                                                __FieldVisitor,
                                            )
                                        }
                                    }
                                    struct __Visitor<'de> {
                                        marker: _serde::__private::PhantomData<QueryMsg>,
                                        lifetime: _serde::__private::PhantomData<&'de ()>,
                                    }
                                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                                        type Value = QueryMsg;
                                        fn expecting(
                                            &self,
                                            __formatter: &mut _serde::__private::Formatter,
                                        ) -> _serde::__private::fmt::Result
                                        {
                                            _serde::__private::Formatter::write_str(
                                                __formatter,
                                                "struct variant QueryMsg::AdminList",
                                            )
                                        }
                                        #[inline]
                                        fn visit_seq<__A>(
                                            self,
                                            _: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::SeqAccess<'de>,
                                        {
                                            _serde::__private::Ok(QueryMsg::AdminList {})
                                        }
                                        #[inline]
                                        fn visit_map<__A>(
                                            self,
                                            mut __map: __A,
                                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                                        where
                                            __A: _serde::de::MapAccess<'de>,
                                        {
                                            while let _serde::__private::Some(__key) =
                                                match _serde::de::MapAccess::next_key::<__Field>(
                                                    &mut __map,
                                                ) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                }
                                            {
                                                match __key {
                                                    _ => {
                                                        let _ =
                                                            match _serde::de::MapAccess::next_value::<
                                                                _serde::de::IgnoredAny,
                                                            >(
                                                                &mut __map
                                                            ) {
                                                                _serde::__private::Ok(__val) => {
                                                                    __val
                                                                }
                                                                _serde::__private::Err(__err) => {
                                                                    return _serde::__private::Err(
                                                                        __err,
                                                                    );
                                                                }
                                                            };
                                                    }
                                                }
                                            }
                                            _serde::__private::Ok(QueryMsg::AdminList {})
                                        }
                                    }
                                    const FIELDS: &'static [&'static str] = &[];
                                    _serde::de::VariantAccess::struct_variant(
                                        __variant,
                                        FIELDS,
                                        __Visitor {
                                            marker: _serde::__private::PhantomData::<QueryMsg>,
                                            lifetime: _serde::__private::PhantomData,
                                        },
                                    )
                                }
                            }
                        }
                    }
                    const VARIANTS: &'static [&'static str] = &["admin_list"];
                    _serde::Deserializer::deserialize_enum(
                        __deserializer,
                        "QueryMsg",
                        VARIANTS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<QueryMsg>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
            }
        };
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for QueryMsg {
            #[inline]
            fn clone(&self) -> QueryMsg {
                match (&*self,) {
                    (&QueryMsg::AdminList {},) => QueryMsg::AdminList {},
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for QueryMsg {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match (&*self,) {
                    (&QueryMsg::AdminList {},) => {
                        let debug_trait_builder =
                            &mut ::core::fmt::Formatter::debug_struct(f, "AdminList");
                        ::core::fmt::DebugStruct::finish(debug_trait_builder)
                    }
                }
            }
        }
        impl ::core::marker::StructuralPartialEq for QueryMsg {}
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for QueryMsg {
            #[inline]
            fn eq(&self, other: &QueryMsg) -> bool {
                match (&*self, &*other) {
                    _ => true,
                }
            }
        }
        const _: () = {
            #[automatically_derived]
            #[allow(unused_braces)]
            impl schemars::JsonSchema for QueryMsg {
                fn schema_name() -> std::string::String {
                    "QueryMsg".to_owned()
                }
                fn json_schema(
                    gen: &mut schemars::gen::SchemaGenerator,
                ) -> schemars::schema::Schema {
                    schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                        subschemas: Some(Box::new(schemars::schema::SubschemaValidation {
                            one_of: Some(<[_]>::into_vec(box [schemars::schema::Schema::Object(
                                schemars::schema::SchemaObject {
                                    instance_type: Some(
                                        schemars::schema::InstanceType::Object.into(),
                                    ),
                                    object: Some(Box::new(schemars::schema::ObjectValidation {
                                        properties: {
                                            let mut props = schemars::Map::new();
                                            props.insert("admin_list".to_owned(), {
                                                let mut schema_object =
                                                    schemars::schema::SchemaObject {
                                                        instance_type: Some(
                                                            schemars::schema::InstanceType::Object
                                                                .into(),
                                                        ),
                                                        ..Default::default()
                                                    };
                                                let object_validation = schema_object.object();
                                                schemars::schema::Schema::Object(schema_object)
                                            });
                                            props
                                        },
                                        required: {
                                            let mut required = schemars::Set::new();
                                            required.insert("admin_list".to_owned());
                                            required
                                        },
                                        additional_properties: Some(Box::new(false.into())),
                                        ..Default::default()
                                    })),
                                    ..Default::default()
                                },
                            )])),
                            ..Default::default()
                        })),
                        ..Default::default()
                    })
                }
            };
        };
        impl QueryMsg {
            pub fn dispatch<C: Whitelist<T>, T>(
                self,
                contract: &C,
                ctx: (cosmwasm_std::Deps, cosmwasm_std::Env),
            ) -> std::result::Result<cosmwasm_std::Binary, C::Error>
            where
                T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
            {
                use QueryMsg::*;
                match self {
                    AdminList {} => cosmwasm_std::to_binary(&contract.admin_list(ctx.into())?)
                        .map_err(Into::into),
                }
            }
        }
    }
}
pub mod msg {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use cosmwasm_std::{
        to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
    };
    use crate::error::ContractError;
    use crate::interfaces::*;
    use crate::state::Cw1WhitelistContract;
    pub use crate::contract::msg::InstantiateMsg;
    pub use crate::interfaces::{cw1, whitelist};
    pub struct AdminListResponse {
        pub admins: Vec<String>,
        pub mutable: bool,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for AdminListResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = match _serde::Serializer::serialize_struct(
                    __serializer,
                    "AdminListResponse",
                    false as usize + 1 + 1,
                ) {
                    _serde::__private::Ok(__val) => __val,
                    _serde::__private::Err(__err) => {
                        return _serde::__private::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "admins",
                    &self.admins,
                ) {
                    _serde::__private::Ok(__val) => __val,
                    _serde::__private::Err(__err) => {
                        return _serde::__private::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "mutable",
                    &self.mutable,
                ) {
                    _serde::__private::Ok(__val) => __val,
                    _serde::__private::Err(__err) => {
                        return _serde::__private::Err(__err);
                    }
                };
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for AdminListResponse {
            fn deserialize<__D>(__deserializer: __D) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum __Field {
                    __field0,
                    __field1,
                    __ignore,
                }
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(__formatter, "field identifier")
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "admins" => _serde::__private::Ok(__Field::__field0),
                            "mutable" => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"admins" => _serde::__private::Ok(__Field::__field0),
                            b"mutable" => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                    }
                }
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<AdminListResponse>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = AdminListResponse;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct AdminListResponse",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match match _serde::de::SeqAccess::next_element::<Vec<String>>(
                            &mut __seq,
                        ) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        } {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(_serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct AdminListResponse with 2 elements",
                                ));
                            }
                        };
                        let __field1 =
                            match match _serde::de::SeqAccess::next_element::<bool>(&mut __seq) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            } {
                                _serde::__private::Some(__value) => __value,
                                _serde::__private::None => {
                                    return _serde::__private::Err(
                                        _serde::de::Error::invalid_length(
                                            1usize,
                                            &"struct AdminListResponse with 2 elements",
                                        ),
                                    );
                                }
                            };
                        _serde::__private::Ok(AdminListResponse {
                            admins: __field0,
                            mutable: __field1,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<Vec<String>> =
                            _serde::__private::None;
                        let mut __field1: _serde::__private::Option<bool> = _serde::__private::None;
                        while let _serde::__private::Some(__key) =
                            match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "admins",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        match _serde::de::MapAccess::next_value::<Vec<String>>(
                                            &mut __map,
                                        ) {
                                            _serde::__private::Ok(__val) => __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "mutable",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
                                        match _serde::de::MapAccess::next_value::<bool>(&mut __map)
                                        {
                                            _serde::__private::Ok(__val) => __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    );
                                }
                                _ => {
                                    let _ = match _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)
                                    {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    };
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                match _serde::__private::de::missing_field("admins") {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                }
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                match _serde::__private::de::missing_field("mutable") {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                }
                            }
                        };
                        _serde::__private::Ok(AdminListResponse {
                            admins: __field0,
                            mutable: __field1,
                        })
                    }
                }
                const FIELDS: &'static [&'static str] = &["admins", "mutable"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "AdminListResponse",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<AdminListResponse>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for AdminListResponse {
        #[inline]
        fn clone(&self) -> AdminListResponse {
            match *self {
                AdminListResponse {
                    admins: ref __self_0_0,
                    mutable: ref __self_0_1,
                } => AdminListResponse {
                    admins: ::core::clone::Clone::clone(&(*__self_0_0)),
                    mutable: ::core::clone::Clone::clone(&(*__self_0_1)),
                },
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for AdminListResponse {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for AdminListResponse {
        #[inline]
        fn eq(&self, other: &AdminListResponse) -> bool {
            match *other {
                AdminListResponse {
                    admins: ref __self_1_0,
                    mutable: ref __self_1_1,
                } => match *self {
                    AdminListResponse {
                        admins: ref __self_0_0,
                        mutable: ref __self_0_1,
                    } => (*__self_0_0) == (*__self_1_0) && (*__self_0_1) == (*__self_1_1),
                },
            }
        }
        #[inline]
        fn ne(&self, other: &AdminListResponse) -> bool {
            match *other {
                AdminListResponse {
                    admins: ref __self_1_0,
                    mutable: ref __self_1_1,
                } => match *self {
                    AdminListResponse {
                        admins: ref __self_0_0,
                        mutable: ref __self_0_1,
                    } => (*__self_0_0) != (*__self_1_0) || (*__self_0_1) != (*__self_1_1),
                },
            }
        }
    }
    const _: () = {
        #[automatically_derived]
        #[allow(unused_braces)]
        impl schemars::JsonSchema for AdminListResponse {
            fn schema_name() -> std::string::String {
                "AdminListResponse".to_owned()
            }
            fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                {
                    let mut schema_object = schemars::schema::SchemaObject {
                        instance_type: Some(schemars::schema::InstanceType::Object.into()),
                        ..Default::default()
                    };
                    let object_validation = schema_object.object();
                    {
                        object_validation
                            .properties
                            .insert("admins".to_owned(), gen.subschema_for::<Vec<String>>());
                        if !<Vec<String> as schemars::JsonSchema>::_schemars_private_is_option() {
                            object_validation.required.insert("admins".to_owned());
                        }
                    }
                    {
                        object_validation
                            .properties
                            .insert("mutable".to_owned(), gen.subschema_for::<bool>());
                        if !<bool as schemars::JsonSchema>::_schemars_private_is_option() {
                            object_validation.required.insert("mutable".to_owned());
                        }
                    }
                    schemars::schema::Schema::Object(schema_object)
                }
            }
        };
    };
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for AdminListResponse {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                AdminListResponse {
                    admins: ref __self_0_0,
                    mutable: ref __self_0_1,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "AdminListResponse");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "admins",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "mutable",
                        &&(*__self_0_1),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
}
pub mod multitest {}
pub mod query {
    use cosmwasm_std::{Addr, CosmosMsg, CustomQuery, QuerierWrapper, StdResult};
    use cw1::CanExecuteResponse;
    use serde::Serialize;
    use crate::msg::AdminListResponse;
    #[must_use]
    pub struct Cw1Querier<'a, C>
    where
        C: CustomQuery,
    {
        addr: &'a Addr,
        querier: &'a QuerierWrapper<'a, C>,
    }
    impl<'a, C> Cw1Querier<'a, C>
    where
        C: CustomQuery,
    {
        pub fn new(addr: &'a Addr, querier: &'a QuerierWrapper<'a, C>) -> Self {
            Self { addr, querier }
        }
        pub fn can_execute(
            &self,
            sender: String,
            msg: CosmosMsg<impl Serialize>,
        ) -> StdResult<CanExecuteResponse> {
            self.querier.query_wasm_smart(
                self.addr.as_str(),
                &msg::cw1::QueryMsg::CanExecute { sender, msg },
            )
        }
    }
    #[must_use]
    pub struct WhitelistQuerier<'a, C>
    where
        C: CustomQuery,
    {
        addr: &'a Addr,
        querier: &'a QuerierWrapper<'a, C>,
    }
    impl<'a, C> WhitelistQuerier<'a, C>
    where
        C: CustomQuery,
    {
        pub fn new(addr: &'a Addr, querier: &'a QuerierWrapper<'a, C>) -> Self {
            Self { addr, querier }
        }
        pub fn admin_list(&self) -> StdResult<AdminListResponse> {
            self.querier
                .query_wasm_smart(self.addr.as_str(), &msg::whitelist::QueryMsg::AdminList {})
        }
    }
}
pub mod state {
    use std::marker::PhantomData;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use cosmwasm_std::{Addr, Empty};
    use cw_storage_plus::Item;
    pub struct AdminList {
        pub admins: Vec<Addr>,
        pub mutable: bool,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for AdminList {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = match _serde::Serializer::serialize_struct(
                    __serializer,
                    "AdminList",
                    false as usize + 1 + 1,
                ) {
                    _serde::__private::Ok(__val) => __val,
                    _serde::__private::Err(__err) => {
                        return _serde::__private::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "admins",
                    &self.admins,
                ) {
                    _serde::__private::Ok(__val) => __val,
                    _serde::__private::Err(__err) => {
                        return _serde::__private::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "mutable",
                    &self.mutable,
                ) {
                    _serde::__private::Ok(__val) => __val,
                    _serde::__private::Err(__err) => {
                        return _serde::__private::Err(__err);
                    }
                };
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for AdminList {
            fn deserialize<__D>(__deserializer: __D) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum __Field {
                    __field0,
                    __field1,
                    __ignore,
                }
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(__formatter, "field identifier")
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "admins" => _serde::__private::Ok(__Field::__field0),
                            "mutable" => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"admins" => _serde::__private::Ok(__Field::__field0),
                            b"mutable" => _serde::__private::Ok(__Field::__field1),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                    }
                }
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<AdminList>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = AdminList;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(__formatter, "struct AdminList")
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match match _serde::de::SeqAccess::next_element::<Vec<Addr>>(
                            &mut __seq,
                        ) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        } {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(_serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct AdminList with 2 elements",
                                ));
                            }
                        };
                        let __field1 =
                            match match _serde::de::SeqAccess::next_element::<bool>(&mut __seq) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            } {
                                _serde::__private::Some(__value) => __value,
                                _serde::__private::None => {
                                    return _serde::__private::Err(
                                        _serde::de::Error::invalid_length(
                                            1usize,
                                            &"struct AdminList with 2 elements",
                                        ),
                                    );
                                }
                            };
                        _serde::__private::Ok(AdminList {
                            admins: __field0,
                            mutable: __field1,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<Vec<Addr>> =
                            _serde::__private::None;
                        let mut __field1: _serde::__private::Option<bool> = _serde::__private::None;
                        while let _serde::__private::Some(__key) =
                            match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "admins",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        match _serde::de::MapAccess::next_value::<Vec<Addr>>(
                                            &mut __map,
                                        ) {
                                            _serde::__private::Ok(__val) => __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "mutable",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
                                        match _serde::de::MapAccess::next_value::<bool>(&mut __map)
                                        {
                                            _serde::__private::Ok(__val) => __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    );
                                }
                                _ => {
                                    let _ = match _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)
                                    {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    };
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                match _serde::__private::de::missing_field("admins") {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                }
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                match _serde::__private::de::missing_field("mutable") {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                }
                            }
                        };
                        _serde::__private::Ok(AdminList {
                            admins: __field0,
                            mutable: __field1,
                        })
                    }
                }
                const FIELDS: &'static [&'static str] = &["admins", "mutable"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "AdminList",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<AdminList>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for AdminList {
        #[inline]
        fn clone(&self) -> AdminList {
            match *self {
                AdminList {
                    admins: ref __self_0_0,
                    mutable: ref __self_0_1,
                } => AdminList {
                    admins: ::core::clone::Clone::clone(&(*__self_0_0)),
                    mutable: ::core::clone::Clone::clone(&(*__self_0_1)),
                },
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for AdminList {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for AdminList {
        #[inline]
        fn eq(&self, other: &AdminList) -> bool {
            match *other {
                AdminList {
                    admins: ref __self_1_0,
                    mutable: ref __self_1_1,
                } => match *self {
                    AdminList {
                        admins: ref __self_0_0,
                        mutable: ref __self_0_1,
                    } => (*__self_0_0) == (*__self_1_0) && (*__self_0_1) == (*__self_1_1),
                },
            }
        }
        #[inline]
        fn ne(&self, other: &AdminList) -> bool {
            match *other {
                AdminList {
                    admins: ref __self_1_0,
                    mutable: ref __self_1_1,
                } => match *self {
                    AdminList {
                        admins: ref __self_0_0,
                        mutable: ref __self_0_1,
                    } => (*__self_0_0) != (*__self_1_0) || (*__self_0_1) != (*__self_1_1),
                },
            }
        }
    }
    const _: () = {
        #[automatically_derived]
        #[allow(unused_braces)]
        impl schemars::JsonSchema for AdminList {
            fn schema_name() -> std::string::String {
                "AdminList".to_owned()
            }
            fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                {
                    let mut schema_object = schemars::schema::SchemaObject {
                        instance_type: Some(schemars::schema::InstanceType::Object.into()),
                        ..Default::default()
                    };
                    let object_validation = schema_object.object();
                    {
                        object_validation
                            .properties
                            .insert("admins".to_owned(), gen.subschema_for::<Vec<Addr>>());
                        if !<Vec<Addr> as schemars::JsonSchema>::_schemars_private_is_option() {
                            object_validation.required.insert("admins".to_owned());
                        }
                    }
                    {
                        object_validation
                            .properties
                            .insert("mutable".to_owned(), gen.subschema_for::<bool>());
                        if !<bool as schemars::JsonSchema>::_schemars_private_is_option() {
                            object_validation.required.insert("mutable".to_owned());
                        }
                    }
                    schemars::schema::Schema::Object(schema_object)
                }
            }
        };
    };
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for AdminList {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                AdminList {
                    admins: ref __self_0_0,
                    mutable: ref __self_0_1,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "AdminList");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "admins",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "mutable",
                        &&(*__self_0_1),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for AdminList {
        #[inline]
        fn default() -> AdminList {
            AdminList {
                admins: ::core::default::Default::default(),
                mutable: ::core::default::Default::default(),
            }
        }
    }
    impl AdminList {
        /// returns true if the address is a registered admin
        pub fn is_admin(&self, addr: &str) -> bool {
            self.admins.iter().any(|a| a.as_ref() == addr)
        }
        /// returns true if the address is a registered admin and the config is mutable
        pub fn can_modify(&self, addr: &str) -> bool {
            self.mutable && self.is_admin(addr)
        }
    }
    pub struct Cw1WhitelistContract<T> {
        pub(crate) admin_list: Item<'static, AdminList>,
        _msg: PhantomData<T>,
    }
    impl Cw1WhitelistContract<Empty> {
        pub const fn native() -> Self {
            Self::new()
        }
    }
    impl<T> Cw1WhitelistContract<T> {
        pub const fn new() -> Self {
            Self {
                admin_list: Item::new("admin_list"),
                _msg: PhantomData,
            }
        }
    }
}
#[cfg(not(feature = "library"))]
mod entry_points {
    use crate::error::ContractError;
    use crate::state::Cw1WhitelistContract;
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};
    const CONTRACT: Cw1WhitelistContract<Empty> = Cw1WhitelistContract::native();
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: crate::msg::InstantiateMsg,
    ) -> Result<Response, ContractError> {
        msg.dispatch(Cw1WhitelistContract::native(), (deps, env, info))
    }
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Binary,
    ) -> Result<Response, ContractError> {
        CONTRACT.entry_execute(deps, env, info, &msg)
    }
    pub fn query(deps: Deps, env: Env, msg: Binary) -> Result<Binary, ContractError> {
        CONTRACT.entry_query(deps, env, &msg)
    }
}
#[cfg(not(feature = "library"))]
pub use entry_points::*;
