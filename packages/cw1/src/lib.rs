pub mod helpers;
pub mod msg;

pub use crate::helpers::{Cw1CanonicalContract, Cw1Contract};
pub use crate::msg::Cw1HandleMsg;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
