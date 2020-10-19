use cosmwasm_std::{Api, Querier, Storage};

///! some features that should be in cosmwasm_std v0.12 mocked out here for ease

pub struct ExternMut<'a, S: Storage, A: Api, Q: Querier> {
    pub storage: &'a mut S,
    pub api: &'a A,
    pub querier: &'a Q,
}

pub struct ExternRef<'a, S: Storage, A: Api, Q: Querier> {
    pub storage: &'a S,
    pub api: &'a A,
    pub querier: &'a Q,
}

impl<'a, S: Storage, A: Api, Q: Querier> ExternMut<'a, S, A, Q> {
    pub fn as_ref(self) -> ExternRef<'a, S, A, Q> {
        ExternRef {
            storage: self.storage,
            api: self.api,
            querier: self.querier,
        }
    }
}

// pub struct Extern<S: Storage, A: Api, Q: Querier> {
//     pub storage: S,
//     pub api: A,
//     pub querier: Q,
// }
//
// impl<S: Storage, A: Api, Q: Querier> Extern<S, A, Q> {
//     pub fn as_ref(&'_ self) -> ExternRef<'_, S, A, Q> {
//         ExternRef {
//             storage: &self.storage,
//             api: &self.api,
//             querier: &self.querier,
//         }
//     }
//
//     pub fn as_mut(&'_ mut self) -> ExternMut<'_, S, A, Q> {
//         ExternMut {
//             storage: &mut self.storage,
//             api: &self.api,
//             querier: &self.querier,
//         }
//     }
// }
