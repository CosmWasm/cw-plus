# Migration Tips

Note, that we currently do not support this fully for external use.
These are some partial tips to help in upgrades.

## 0.7 -> 0.8

* `SimpleBank` was renamed to `BankKeeper`
* `App::new` takes `Box<dyn Storage>` rather than a closure as the last argument.
* Your test setup will look something like this:

  ```rust
  pub fn mock_app() -> App<Empty> {
    let env = mock_env();
    let api = Box::new(MockApi::default());
    let bank = BankKeeper::new();

    App::new(api, env.block, bank, Box::new(MockStorage::new()))
  }
  ```
* `App.set_bank_balance` was renamed to `init_bank_balance`, with the same args.
* You will want to import `cw_multi_test::Executor` in order to get access to the execution helpers
  like `App.execute_contract`, and `App.instantiate_contract`
* `App.instantiate_contract` takes one additional arg: `admin: Option<String>`. You can set it to `None`
  unless you want to test migrations.
