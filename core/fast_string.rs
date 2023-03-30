// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use std::borrow::{Borrow, Cow};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use url::Url;
use v8::NewStringType;

/// Module code can be sourced from strings or bytes that are either owned or borrowed. This enumeration allows us
/// to perform a minimal amount of cloning and format-shifting of the underlying data.
///
/// Note that any [`ModuleCode`] created from a `'static` byte array or string must contain ASCII characters.
///
/// Examples of ways to construct a [`ModuleCode`] object:
///
/// ```rust
/// # use deno_core::ModuleCode;
///
/// let code: ModuleCode = "a string".into();
/// let code: ModuleCode = b"a string".into();
/// ```
#[derive(Clone)]
pub enum FastString {
  /// Created from static data.
  Static(&'static str),

  /// Created from static data, known to contain only ASCII chars.
  StaticAscii(&'static str),

  // Scripts loaded from the `deno_graph` infrastructure.
  Arc(Arc<str>),
}

pub trait IsPotentiallyOwned {
  fn maybe_into_owned_vec(self) -> Cow<'static, str>;
}

impl IsPotentiallyOwned for String {
  fn maybe_into_owned_vec(self) -> Cow<'static, str> {
    self.into()
  }
}

impl FastString {
  /// Compiler-time function to determine if a string is ASCII. Note that UTF-8 chars
  /// longer than one byte have the high-bit set and thus, are not ASCII.
  const fn is_ascii(s: &'static [u8]) -> bool {
    let mut i = 0;
    while i < s.len() {
      if !s[i].is_ascii() {
        return false;
      }
      i += 1;
    }
    true
  }

  pub const fn from_static(s: &'static str) -> Self {
    if Self::is_ascii(s.as_bytes()) {
      Self::StaticAscii(s)
    } else {
      Self::Static(s)
    }
  }

  pub const fn ensure_static_ascii(s: &'static str) -> Self {
    if Self::is_ascii(s.as_bytes()) {
      Self::StaticAscii(s)
    } else {
      panic!("This string contained non-ASCII characters and cannot be created with ensure_static_ascii")
    }
  }

  pub fn from_ownable(s: impl IsPotentiallyOwned) -> Self {
    let s = s.maybe_into_owned_vec();
    match s {
      Cow::Owned(s) => Self::Arc(s.into_boxed_str().into()),
      Cow::Borrowed(s) => Self::from_static(s),
    }
  }

  pub fn from_arc(s: Arc<str>) -> Self {
    Self::Arc(s)
  }

  pub const fn try_static_ascii(&self) -> Option<&'static [u8]> {
    match self {
      Self::StaticAscii(s) => Some(s.as_bytes()),
      _ => None,
    }
  }

  pub fn as_bytes(&self) -> &[u8] {
    // TODO(mmastrac): This can be const eventually
    match self {
      Self::Arc(s) => s.as_bytes(),
      Self::Static(s) => s.as_bytes(),
      Self::StaticAscii(s) => s.as_bytes(),
    }
  }

  pub fn as_str(&self) -> &str {
    // TODO(mmastrac): This can be const eventually
    match self {
      Self::Arc(s) => s,
      Self::Static(s) => s,
      Self::StaticAscii(s) => s,
    }
  }

  pub fn v8<'a>(
    &self,
    scope: &mut v8::HandleScope<'a>,
  ) -> v8::Local<'a, v8::String> {
    match self.try_static_ascii() {
      Some(s) => v8::String::new_external_onebyte_static(scope, s).unwrap(),
      None => {
        v8::String::new_from_utf8(scope, self.as_bytes(), NewStringType::Normal)
          .unwrap()
      }
    }
  }

  /// Truncates a `ModuleCode`] value, possibly re-allocating or memcpy'ing. May be slow.
  pub fn truncate(&mut self, index: usize) {
    match self {
      Self::Static(b) => *self = Self::Static(&b[..index]),
      Self::StaticAscii(b) => *self = Self::StaticAscii(&b[..index]),
      // We can't do much if we have an Arc<str>, so we'll just take ownership of the truncated version
      Self::Arc(s) => *self = s[..index].to_owned().into(),
    }
  }
}

impl Hash for FastString {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.as_str().hash(state)
  }
}

impl AsRef<str> for FastString {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl Borrow<str> for FastString {
  fn borrow(&self) -> &str {
    self.as_str()
  }
}

impl Debug for FastString {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(self.as_str(), f)
  }
}

impl Default for FastString {
  fn default() -> Self {
    Self::StaticAscii("")
  }
}

impl PartialEq for FastString {
  fn eq(&self, other: &Self) -> bool {
    self.as_bytes() == other.as_bytes()
  }
}

impl Eq for FastString {}

/// [`FastString`] can be make cheaply from [`Url`] as we know it's owned and don't need to do an
/// ASCII check.
impl From<Url> for FastString {
  fn from(value: Url) -> Self {
    let s: String = value.into();
    s.into()
  }
}

/// [`FastString`] can be make cheaply from [`String`] as we know it's owned and don't need to do an
/// ASCII check.
impl From<String> for FastString {
  fn from(value: String) -> Self {
    FastString::Arc(value.into_boxed_str().into())
  }
}

/// Include a fast string in the binary.
#[macro_export]
macro_rules! include_fast_string {
  ($file:literal) => {
    $crate::FastString::ensure_static_ascii(include_str!($file))
  };
}

#[macro_export]
macro_rules! fast {
  ($str:literal) => {
    $crate::FastString::ensure_static_ascii($str)
  };
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn truncate() {
    let mut s = "123456".to_owned();
    s.truncate(3);

    let mut code: FastString = FastString::from_static("123456");
    code.truncate(3);
    assert_eq!(s, code.as_ref());

    let mut code: FastString = FastString::from_ownable("123456".to_owned());
    code.truncate(3);
    assert_eq!(s, code.as_ref());

    let arc_str: Arc<str> = "123456".into();
    let mut code: FastString = FastString::from_arc(arc_str);
    code.truncate(3);
    assert_eq!(s, code.as_ref());
  }
}