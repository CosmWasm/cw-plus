use cosmwasm_std::Addr;

#[derive(Debug, Clone, Copy)]
pub struct AddrRef<'a>(&'a str);

impl<'a> From<&'a Addr> for AddrRef<'a> {
    fn from(addr: &'a Addr) -> Self {
        AddrRef(addr.as_ref())
    }
}

impl<'a> AddrRef<'a> {
    pub fn new(addr: &'a Addr) -> Self {
        AddrRef(addr.as_ref())
    }

    pub fn unchecked(addr: &'a str) -> Self {
        AddrRef(addr)
    }

    pub fn as_str(&self) -> &str {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn to_owned(&self) -> Addr {
        Addr::unchecked(self.0.to_string())
    }
}
