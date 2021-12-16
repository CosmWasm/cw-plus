use cosmwasm_std::Response;

struct Ctx;
struct Error;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
struct Member;

#[cw_derive::interface(module=msg, exec=Execute, query=Query)]
trait Cw4 {
    #[msg(exec)]
    fn update_admin(&self, ctx: Ctx, admin: Option<String>) -> Result<Response, Error>;

    #[msg(exec)]
    fn update_members(
        &self,
        ctx: Ctx,
        remove: Vec<String>,
        add: Vec<Member>,
    ) -> Result<Response, Error>;

    #[msg(exec)]
    fn add_hook(&self, ctx: Ctx, addr: String) -> Result<Response, Error>;

    #[msg(exec)]
    fn remove_hook(&self, ctx: Ctx, addr: String) -> Result<Response, Error>;
}

fn main() {}
