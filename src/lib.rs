//! A wrapper around the standard [`std::env`](https://doc.rust-lang.org/std/env/index.html)
//! functions that allows for a test double to be injected during testing.
//!
//! # Motivation
//! Testing code that relies on the state of environment variables can be
//! fragile, since the state may change between tests or be polluted by other tests.
//! The ideal solution is to have a private set of environment variables per test,
//! so these problems cannot happen.
//!
//! # Approach
//! This crate introduces the [`RealEnvironment`](RealEnvironment)
//! (a wrapper around the functions in [`std::env`](https://doc.rust-lang.org/std/env/index.html))
//! and
//! [`FakeEnvironment`](FakeEnvironment) structs, which implement the
//! [`Environment`](Environment) trait. Instead of using
//! [`std::env`](https://doc.rust-lang.org/std/env/index.html) directly,
//! use [`RealEnvironment`](RealEnvironment) with
//! [dependency injection](https://en.wikipedia.org/wiki/Dependency_injection)
//! so each of your tests can have a private set of environment variables.
//!
//! # Example
//! Scenario: An app looks for the presence of the `CONFIG_LOCATION` environment
//! variable. If it isn't set, it uses a default location.
//!
//! ```rust
//! use env_wrapper::{Environment, RealEnvironment};
//!
//! const CONFIG_LOCATION_ENV_VAR_NAME: &str = "CONFIG_LOCATION";
//! const DEFAULT_CONFIG_LOCATION: &str = "/etc/my_app/service.conf";
//!
//! fn main() {
//!     // In the production code, inject RealEnvironment.
//!     let real_env = RealEnvironment;
//!     let config_location = get_config_location(real_env);
//! }
//!
//! fn get_config_location(env: impl Environment) -> String {
//!     match env.var(CONFIG_LOCATION_ENV_VAR_NAME) {
//!         Ok(location) => location,
//!         _ => DEFAULT_CONFIG_LOCATION.to_string(),
//!     }
//! }
//!
//! #[test]
//! fn when_the_user_has_set_the_config_location_env_var_then_use_that_location() {
//!     use env_wrapper::FakeEnvironment;
//!
//!     // Arrange
//!     // Each test should have a separate instance of FakeEnvironment.
//!     let mut fake_env = FakeEnvironment::new();
//!     let user_specified_location = "/a/user/specified/location";
//!     fake_env.set_var(CONFIG_LOCATION_ENV_VAR_NAME, user_specified_location);
//!     
//!     // Act
//!     // In the test code, inject FakeEnvironment.
//!     let location = get_config_location(fake_env);
//!
//!     // Assert
//!     assert_eq!(location, user_specified_location);
//! }
//! ```

#[cfg(test)]
pub(crate) mod test_helpers;

use std::{
    collections::HashMap,
    env::{self, VarError},
    ffi::{OsStr, OsString},
};

/// Represents a process's environment.
pub trait Environment {
    /// Set an environment variable.
    fn set_var(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>);

    /// Get an environment variable, checking for valid UTF-8. If valid UTF-8
    /// checks are not needed, use `var_os`.
    ///
    /// # Errors
    /// * If a key doesn't exist, it should return a `VarError::NotPresent`.
    /// * If the environment variable value contains invalid UTF-8, it
    /// should return `VarError::NotUnicode(OsString)`.
    fn var(&self, key: impl AsRef<OsStr>) -> Result<String, VarError>;

    /// Get an environment variable. This does not check for valid UTF-8.
    /// If a valid UTF-8 check is needed, use `var` instead.
    fn var_os(&self, key: impl AsRef<OsStr>) -> Option<OsString>;

    /// Remove an environment variable from the current process environment.
    fn remove_var(&mut self, key: impl AsRef<OsStr>);
}

/// The process's environment. Wraps the standard
/// [`std::env`](https://doc.rust-lang.org/std/env/index.html) functions.
///
/// When testing, [`FakeEnvironment`](FakeEnvironment) should likely be used instead.
///
/// # Note
/// Different instances of the struct all reference the _same_ underlying process
/// environment.
///
/// # Example
/// ```rust
/// # use env_wrapper::{Environment, RealEnvironment};
/// let real_env = RealEnvironment;
/// get_config_location(real_env);
///
/// fn get_config_location(env: impl Environment) {
/// //...
/// }
/// ```
pub struct RealEnvironment;

impl Environment for RealEnvironment {
    /// From [`std::env::set_var`](https://doc.rust-lang.org/std/env/fn.set_var.html):
    /// > Sets the environment variable `key` to the value `value` for the currently running
    /// > process.
    /// >
    /// > Note that while concurrent access to environment variables is safe in Rust,
    /// > some platforms only expose inherently unsafe non-threadsafe APIs for
    /// > inspecting the environment. As a result, extra care needs to be taken when
    /// > auditing calls to unsafe external FFI functions to ensure that any external
    /// > environment accesses are properly synchronized with accesses in Rust.
    /// >
    /// > Discussion of this unsafety on Unix may be found in:
    /// >
    /// >  - [Austin Group Bugzilla](https://austingroupbugs.net/view.php?id=188)
    /// >  - [GNU C library Bugzilla](https://sourceware.org/bugzilla/show_bug.cgi?id=15607#c2)
    /// >
    /// > # Panics
    /// >
    /// > This function may panic if `key` is empty, contains an ASCII equals sign `'='`
    /// > or the NUL character `'\0'`, or when `value` contains the NUL character.
    fn set_var(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        env::set_var(key, value)
    }

    /// From [`std::env::var`](https://doc.rust-lang.org/std/env/fn.var.html):
    /// > Fetches the environment variable `key` from the current process.
    /// >
    /// > # Errors
    /// >
    /// > This function will return an error if the environment variable isn't set.
    /// >
    /// > This function may return an error if the environment variable's name contains
    /// > the equal sign character (`=`) or the NUL character.
    /// >
    /// > This function will return an error if the environment variable's value is
    /// > not valid Unicode. If this is not desired, consider using [`var_os`].
    fn var(&self, key: impl AsRef<OsStr>) -> Result<String, VarError> {
        env::var(key)
    }

    /// From [`std::env::var_os`](https://doc.rust-lang.org/std/env/fn.var_os.html):
    /// > Fetches the environment variable `key` from the current process, returning
    /// > [`None`] if the variable isn't set or there's another error.
    /// >
    /// > Note that the method will not check if the environment variable
    /// > is valid Unicode. If you want to have an error on invalid UTF-8,
    /// > use the [`var`] function instead.
    /// >
    /// > # Errors
    /// >
    /// > This function returns an error if the environment variable isn't set.
    /// >
    /// > This function may return an error if the environment variable's name contains
    /// > the equal sign character (`=`) or the NUL character.
    /// >
    /// > This function may return an error if the environment variable's value contains
    /// > the NUL character.
    fn var_os(&self, key: impl AsRef<OsStr>) -> Option<OsString> {
        env::var_os(key)
    }

    /// From [`std::env::remove_var`](https://doc.rust-lang.org/std/env/fn.remove_var.html):
    /// > Removes an environment variable from the environment of the currently running process.
    /// >
    /// > Note that while concurrent access to environment variables is safe in Rust,
    /// > some platforms only expose inherently unsafe non-threadsafe APIs for
    /// > inspecting the environment. As a result extra care needs to be taken when
    /// > auditing calls to unsafe external FFI functions to ensure that any external
    /// > environment accesses are properly synchronized with accesses in Rust.
    /// >
    /// > Discussion of this unsafety on Unix may be found in:
    /// >
    /// >  - [Austin Group Bugzilla](https://austingroupbugs.net/view.php?id=188)
    /// >  - [GNU C library Bugzilla](https://sourceware.org/bugzilla/show_bug.cgi?id=15607#c2)
    /// >
    /// > # Panics
    /// >
    /// > This function may panic if `key` is empty, contains an ASCII equals sign
    /// > `'='` or the NUL character `'\0'`, or when the value contains the NUL
    /// > character.
    fn remove_var(&mut self, key: impl AsRef<OsStr>) {
        env::remove_var(key)
    }
}

/// A fake process environment, suitable for testing.
///
/// # Notes
/// To make sure one test's environment state does not affect another, use a new
/// instance of `FakeEnvironment` for each test.
///
/// # Example
/// ```rust
/// # use env_wrapper::{Environment, FakeEnvironment};
/// const CONFIG_LOCATION_ENV_VAR_NAME: &str = "CONFIG_LOCATION";
/// const DEFAULT_CONFIG_LOCATION: &str = "/etc/my_app/service.conf";
///
/// fn get_config_location(env: impl Environment) -> String {
///     match env.var(CONFIG_LOCATION_ENV_VAR_NAME) {
///         Ok(location) => location,
///         _ => DEFAULT_CONFIG_LOCATION.to_string(),
///     }
/// }
///
/// #[test]
/// fn when_the_user_has_set_the_config_location_env_var_then_use_that_location() {
///
///     // Arrange
///     // Each test should have a separate instance of FakeEnvironment.
///     let mut fake_env = FakeEnvironment::new();
///     let user_specified_location = "/a/user/specified/location";
///     fake_env.set_var(CONFIG_LOCATION_ENV_VAR_NAME, user_specified_location);
///     
///     // Act
///     // In test code, inject FakeEnvironment.
///     let location = get_config_location(fake_env);
///
///     // Assert
///     assert_eq!(location, user_specified_location);
/// }
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FakeEnvironment {
    env_vars: HashMap<OsString, OsString>,
}

impl FakeEnvironment {
    pub fn new() -> Self {
        FakeEnvironment {
            env_vars: HashMap::new(),
        }
    }
}

impl Environment for FakeEnvironment {
    fn set_var(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        self.env_vars
            .insert(key.as_ref().into(), value.as_ref().into());
    }

    fn var(&self, key: impl AsRef<OsStr>) -> Result<String, VarError> {
        match self.env_vars.get(key.as_ref()) {
            Some(val) => match val.to_str() {
                Some(valid_utf8) => Ok(valid_utf8.into()),
                None => Err(VarError::NotUnicode(val.into())),
            },
            None => Err(VarError::NotPresent),
        }
    }

    fn var_os(&self, key: impl AsRef<OsStr>) -> Option<OsString> {
        self.env_vars.get(key.as_ref()).cloned()
    }

    fn remove_var(&mut self, key: impl AsRef<OsStr>) {
        self.env_vars.remove(key.as_ref());
    }
}

// These tests represent behavior that should be shared by fake and real
// implementations. Both are being tested to enforce behavioral parity.
#[cfg(test)]
mod tests {
    use std::{
        env::VarError,
        ffi::{OsStr, OsString},
        os::unix::ffi::OsStrExt,
    };

    use crate::{test_helpers::random_upper, Environment, FakeEnvironment, RealEnvironment};

    const INVALID_UTF8: [u8; 4] = [0x66, 0x6f, 0x80, 0x6f];

    #[test]
    fn when_adding_an_environment_variable_then_it_can_be_read() {
        fn test(mut env: impl Environment) {
            // Arrange
            let key = random_upper();
            let value = random_upper();
            env.set_var(&key, &value);

            // Act
            let result = env.var(&key);

            // Assert
            assert_eq!(result.unwrap(), value);
        }
        test(RealEnvironment);
        test(FakeEnvironment::new());
    }

    #[test]
    fn given_a_nonexistent_env_var_when_getting_the_env_var_with_var_then_it_is_a_not_present_error(
    ) {
        fn test(env: impl Environment) {
            // Arrange
            let nonexistent_key = random_upper();

            // Act
            let result = env.var(nonexistent_key);

            // Assert
            assert_eq!(result.unwrap_err(), VarError::NotPresent);
        }
        test(RealEnvironment);
        test(FakeEnvironment::new());
    }

    #[test]
    fn when_setting_env_vars_then_multiple_data_types_can_be_used_on_the_same_environment_instance()
    {
        fn test(mut env: impl Environment) {
            // Act
            env.set_var(&random_upper(), &random_upper());
            env.set_var(random_upper(), random_upper());
            env.set_var(OsStr::new(&random_upper()), OsStr::new(&random_upper()));
            env.set_var(
                OsString::from(random_upper()),
                OsString::from(random_upper()),
            );

            // Assert - none. This is strictly for type enforcement.
        }
        test(RealEnvironment);
        test(FakeEnvironment::new());
    }

    #[test]
    fn when_using_var_getter_with_an_invalid_utf8_value_then_it_is_a_not_unicode_error() {
        fn test(mut env: impl Environment) {
            // Arrange
            let key = random_upper();
            env.set_var(&key, OsStr::from_bytes(&INVALID_UTF8));

            // Act
            let result = env.var(&key);

            // Assert
            assert!(matches!(result, Err(VarError::NotUnicode(_))));
        }
        test(RealEnvironment);
        test(FakeEnvironment::new());
    }

    #[test]
    fn given_a_nonexistent_env_var_when_getting_the_env_var_with_var_os_then_it_is_none() {
        fn test(env: impl Environment) {
            // Arrange
            let key = random_upper();

            // Act
            let result = env.var_os(key);

            // Assert
            assert!(result.is_none());
        }
        test(RealEnvironment);
        test(FakeEnvironment::new());
    }

    #[test]
    fn given_an_env_var_with_invalid_utf8_when_getting_the_env_var_with_var_os_then_it_is_some() {
        fn test(mut env: impl Environment) {
            // Arrange
            let key = random_upper();
            env.set_var(&key, OsStr::from_bytes(&INVALID_UTF8));

            // Act
            let result = env.var_os(&key);

            // Assert
            assert!(result.is_some());
        }
        test(RealEnvironment);
        test(FakeEnvironment::new());
    }

    #[test]
    fn given_an_existing_environment_variable_when_setting_the_same_environment_variable_then_the_value_is_overwritten(
    ) {
        fn test(mut env: impl Environment) {
            // Arrange
            let key = random_upper();
            let val_1 = random_upper();
            let val_2 = random_upper();
            env.set_var(&key, &val_1);

            // Act
            env.set_var(&key, &val_2);

            // Assert
            assert_eq!(env.var(&key).unwrap(), val_2);
        }
        test(RealEnvironment);
        test(FakeEnvironment::new());
    }

    #[test]
    fn given_an_existing_environment_variable_when_removing_the_same_environment_variable_then_the_variable_no_longer_exists(
    ) {
        fn test(mut env: impl Environment) {
            // Arrange
            let key = random_upper();
            let value = random_upper();
            env.set_var(&key, &value);

            // Act
            env.remove_var(&key);

            // Assert
            assert_eq!(env.var(&key).unwrap_err(), VarError::NotPresent);
        }
        test(RealEnvironment);
        test(FakeEnvironment::new());
    }

    #[test]
    fn when_removing_a_nonexistent_environment_variable_then_do_not_panic() {
        fn test(mut env: impl Environment) {
            // Arrange
            let key = random_upper();

            // Act
            env.remove_var(&key);

            // Assert - no assertion
        }

        test(RealEnvironment);
        test(FakeEnvironment::new());
    }
}
