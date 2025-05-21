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
    collections::{BTreeMap, HashMap},
    env::{self, VarError},
    ffi::{OsStr, OsString},
};

use regex::RegexSet;

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
        unsafe { env::set_var(key, value) }
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
        unsafe { env::remove_var(key) }
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

/// EnvVarSnapshot stores a copy of environment variables for later recall. The
/// API provided by EnvVarSnapshot is meant to encourage use of environment
/// variables as immutable after the process starts. The motivation is to
/// avoid two classes of bugs associated with environment variables.
///
/// The first is that environment variables are not meant to change within the
/// lifetime of a process except when preparing the environment before spawning
/// new child process. Checking an environment variable at two different times
/// is error prune due to time-of-get vs time-of-set issues. It is safest to
/// avoid any part of our program getting the environment variables set by
/// another part of the same program.
///
/// The second is that accessing environment variables can be fundamentally
/// unsafe. This is particularly true in the presence of multiple threads. It
/// is made worse by dependencies being able to spawn threads and set and get
/// environment variables.
///   * <https://rachelbythebay.com/w/2017/01/30/env/>
///   * <https://www.geldata.com/blog/c-stdlib-isn-t-threadsafe-and-even-safe-rust-didn-t-save-us>
///   * <https://www.evanjones.ca/setenv-is-not-thread-safe.html>
///
/// However, by snapshotting the environment as early as possible and only
/// consulting the snapshot, we can avoid triggering most of these bugs.
#[derive(Clone)]
pub struct EnvVarSnapshot(#[doc(hidden)] BTreeMap<OsString, OsString>);

pub fn as_anchored_pattern(s: &str) -> String {
    format!(r#"\A{}\z"#, s)
}

impl EnvVarSnapshot {
    /// Produces an empty snapshot. Mainly for test scenarios. Intended for testing scenarios.
    pub fn empty() -> Self {
        EnvVarSnapshot(BTreeMap::new())
    }

    /// Produces a snapshot of the named environment variables.
    pub fn new(names: &[&str]) -> Self {
        let mut storage = BTreeMap::new();
        for name in names {
            let key = OsString::from(name);
            if let Some(val) = std::env::var_os(&key) {
                storage.insert(key, val.clone());
            }
        }
        EnvVarSnapshot(storage)
    }

    /// Produces a snapshot of the subset of the environment variables that match the given regex
    /// set. Since regex matching does not match against OsString, this may miss non-UTF-8
    /// variables that would otherwise match the regex.
    pub fn new_from_patterns(patterns: &RegexSet) -> Self {
        let mut storage = BTreeMap::new();
        // Even though we will ignore variable names that cannot be converted to UTF-8, we need to
        // rely on vars_os() here because vars() panics. See:
        // https://doc.rust-lang.org/src/std/env.rs.html#157-165
        Self::copy_matches(std::env::vars_os(), &mut storage, patterns);
        EnvVarSnapshot(storage)
    }

    /// Produces a snapshot with all environment variables present at the time it was called.
    pub fn all_vars() -> Self {
        let mut storage = BTreeMap::new();
        for (name, value) in std::env::vars_os() {
            storage.insert(name.clone(), value.clone());
        }
        EnvVarSnapshot(storage)
    }

    /// Produces a snapshot with the given variables and values. Intended for testing scenarios.
    pub fn new_with_values<'a>(
        pairs: impl IntoIterator<Item = &'a (impl AsRef<OsStr> + 'a, impl AsRef<OsStr> + 'a)>,
    ) -> Self {
        let mut storage = BTreeMap::new();
        for (name, value) in pairs {
            storage.insert(name.into(), value.into());
        }
        EnvVarSnapshot(storage)
    }

    /// Returns an iterator of variable name and value pairs for all variables that were present at
    /// the time the snapshot was taken.
    pub fn present(&self) -> impl Iterator<Item = (String, String)> {
        self.0.iter().filter_map(|(key, value)| {
            match (key.clone().into_string(), value.clone().into_string()) {
                (Ok(k), Ok(v)) => Some((k, v)),
                _ => None,
            }
        })
    }

    /// Produces a new snapshot containing a subset of the environment variables in this snapshot
    /// that match the given regex set.
    pub fn subset_from_patterns(&self, patterns: &RegexSet) -> Self {
        let mut storage = BTreeMap::new();
        Self::copy_matches(self.0.iter(), &mut storage, patterns);
        EnvVarSnapshot(storage)
    }

    fn copy_matches(
        src: impl Iterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
        dst: &mut BTreeMap<OsString, OsString>,
        patterns: &RegexSet,
    ) {
        for (os_name, os_value) in src {
            if let Some(name) = os_name.as_ref().to_str() {
                if patterns.is_match(&name) {
                    dst.insert(
                        os_name.as_ref().to_os_string(),
                        os_value.as_ref().to_os_string(),
                    );
                }
            }
        }
    }
}

impl Environment for EnvVarSnapshot {
    /// Provided for conformance with the Environment trait but always panics because
    /// EnvVarSnapshot should be considered immutable.
    fn set_var(&mut self, _key: impl AsRef<OsStr>, _value: impl AsRef<OsStr>) {
        panic!("Do not call EnvVarSnapshot::set_var");
    }

    /// Get an environment variable, checking for valid UTF-8. If valid UTF-8
    /// checks are not needed, use `var_os`.
    ///
    /// # Errors
    /// * If a key doesn't exist, it should return a `VarError::NotPresent`.
    /// * If the environment variable value contains invalid UTF-8, it
    /// should return `VarError::NotUnicode(OsString)`.
    fn var(&self, key: impl AsRef<OsStr>) -> Result<String, VarError> {
        let Some(os_val) = self.0.get(key.as_ref()) else {
            return Err(VarError::NotPresent);
        };

        let recoded = os_val
            .clone()
            .into_string()
            .map_err(|os| VarError::NotUnicode(os))?;
        Ok(recoded)
    }

    /// Get an environment variable. This does not check for valid UTF-8.
    /// If a valid UTF-8 check is needed, use `var` instead.
    fn var_os(&self, key: impl AsRef<OsStr>) -> Option<OsString> {
        self.0.get(key.as_ref()).cloned()
    }

    /// Provided for conformance with the Environment trait but always panics because
    /// EnvVarSnapshot should be considered immutable.
    fn remove_var(&mut self, _key: impl AsRef<OsStr>) {
        panic!("Do not call EnvVarSnapshot::remove_var");
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

    use regex::RegexSet;

    use crate::{
        test_helpers::random_upper, EnvVarSnapshot, Environment, FakeEnvironment, RealEnvironment,
    };

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
        // EnvVarSnapshot intentionally omitted
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
        test(EnvVarSnapshot::empty());
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
        // EnvVarSnapshot intentionally omitted
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
    fn when_using_var_getter_with_an_invalid_utf8_value_in_snapshot_then_it_is_a_not_unicode_error()
    {
        // Arrange
        let key = OsString::from(random_upper());
        let env = EnvVarSnapshot::new_with_values(&[(&key, OsStr::from_bytes(&INVALID_UTF8))]);

        // Act
        let result = env.var(&key);

        // Assert
        assert!(matches!(result, Err(VarError::NotUnicode(_))));
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
        test(EnvVarSnapshot::empty());
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
    fn given_an_env_var_with_invalid_utf8_in_snapshot_when_getting_the_env_var_with_var_os_then_it_is_some(
    ) {
        // Arrange
        let key = OsString::from(random_upper());
        let env = EnvVarSnapshot::new_with_values(&[(&key, OsStr::from_bytes(&INVALID_UTF8))]);

        // Act
        let result = env.var_os(&key);

        // Assert
        assert!(result.is_some());
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
        // EnvVarSnapshot intentionally omitted
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
        // EnvVarSnapshot intentionally omitted
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
        // EnvVarSnapshot intentionally omitted
    }

    #[test]
    fn given_a_snapshot_of_a_cargo_env_var_it_is_present() {
        let env = EnvVarSnapshot::new(&["CARGO_PKG_VERSION"]);
        assert!(env.var("CARGO_PKG_VERSION").is_ok());
    }

    #[test]
    fn given_a_snapshot_of_all_vars_standard_cargo_envs_are_present() {
        let env = EnvVarSnapshot::all_vars();
        assert!(env.var("CARGO_PKG_VERSION").is_ok());
        assert_eq!(
            env.present()
                .filter(|(name, _value)| { name == "CARGO_PKG_VERSION" })
                .count(),
            1
        );
    }

    #[test]
    fn given_a_snapshot_of_all_cargo_vars_they_are_each_available() {
        let env = EnvVarSnapshot::new_from_patterns(&RegexSet::new(&["^CARGO_"]).unwrap());
        assert_eq!(env.var("CARGO_PKG_VERSION").unwrap(), "0.2.0");
        assert!(!env.var("CARGO_HOME").unwrap().is_empty());
    }

    #[test]
    fn given_a_snapshot_a_disjoint_subset_does_not_contain_them() {
        let env = EnvVarSnapshot::new_from_patterns(&RegexSet::new(&["^CARGO_"]).unwrap());
        assert_eq!(env.var("CARGO_PKG_VERSION").unwrap(), "0.2.0");
        let subset = env.subset_from_patterns(&RegexSet::new(&["NOT_"]).unwrap());
        assert_eq!(
            subset.var("CARGO_PKG_VERSION").unwrap_err(),
            VarError::NotPresent
        );
    }

    #[test]
    fn given_an_empty_env_any_fetch_returns_err() {
        let env = EnvVarSnapshot::empty();
        assert!(env.var("anything").is_err());
    }

    #[test]
    fn given_an_env_with_an_assigned_value_it_is_returned() {
        let env = EnvVarSnapshot::new_with_values(&[(
            &OsString::from("HOME"),
            &OsString::from("/home/myuser"),
        )]);
        assert_eq!(env.var("HOME").unwrap(), "/home/myuser");
    }

    #[test]
    fn given_an_env_snapshot_it_can_be_cloned() {
        let env = EnvVarSnapshot::new_with_values(&[(
            &OsString::from("HOME"),
            &OsString::from("/home/myuser"),
        )]);
        let _ = env.clone();
    }
}
