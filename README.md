# env_wrapper

[![Build](https://github.com/Aembit/env_wrapper/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/Aembit/env_wrapper/actions/workflows/build.yml)

A wrapper around the standard [`std::env`](https://doc.rust-lang.org/std/env/index.html)
functions that allows for a test double to be injected during testing.

## Limitations

At this time, this crate only works for Unix-like systems.

## Motivation

Testing code that relies on the state of environment variables can be
fragile, since the state may change between tests or be polluted by other tests.
The ideal solution is to have a private set of environment variables per test,
so these problems cannot happen.

## Approach

This crate introduces the `RealEnvironment`
(a wrapper around the functions in [`std::env`](https://doc.rust-lang.org/std/env/index.html))
and
`FakeEnvironment` structs, which implement the
`Environment` trait. Instead of using
[`std::env`](https://doc.rust-lang.org/std/env/index.html) directly,
use `RealEnvironment` with
[dependency injection](https://en.wikipedia.org/wiki/Dependency_injection)
so each of your tests can have a private set of environment variables.

## Example

Scenario: An app looks for the presence of the `CONFIG_LOCATION` environment
variable. If it isn't set, it uses a default location.

```rust
use env_wrapper::{Environment, RealEnvironment};

const CONFIG_LOCATION_ENV_VAR_NAME: &str = "CONFIG_LOCATION";
const DEFAULT_CONFIG_LOCATION: &str = "/etc/my_app/service.conf";

fn main() {
    // In the production code, inject RealEnvironment.
    let real_env = RealEnvironment;
    let config_location = get_config_location(real_env);
}

fn get_config_location(env: impl Environment) -> String {
    match env.var(CONFIG_LOCATION_ENV_VAR_NAME) {
        Ok(location) => location,
        _ => DEFAULT_CONFIG_LOCATION.to_string(),
    }
}

#[test]
fn when_the_user_has_set_the_config_location_env_var_then_use_that_location() {
    use env_wrapper::FakeEnvironment;

    // Arrange
    // Each test should have a separate instance of FakeEnvironment.
    let mut fake_env = FakeEnvironment::new();
    let user_specified_location = "/a/user/specified/location";
    fake_env.set_var(CONFIG_LOCATION_ENV_VAR_NAME, user_specified_location);
    
    // Act
    // In the test code, inject FakeEnvironment.
    let location = get_config_location(fake_env);

    // Assert
    assert_eq!(location, user_specified_location);
}
```

## License

Licensed under either of

* Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Code of Conduct
All behavior is governed by the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).
