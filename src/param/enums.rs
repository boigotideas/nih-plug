//! Enum parameters. `enum` is a keyword, so `enums` it is.

use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::Arc;

use super::internals::ParamPtr;
use super::range::Range;
use super::{IntParam, Param};

// Re-export the derive macro
pub use nih_plug_derive::Enum;

/// An enum usable with `EnumParam`. This trait can be derived. Variants are identified by their
/// **declaration order**. You can freely rename the variant names, but reordering them will break
/// compatibility with existing presets. The variatn's name is used as the display name by default.
/// If you want to override this, for instance, because it needs to contain spaces, then yo ucan use
/// the `$[name = "..."]` attribute:
///
/// ```
/// #[derive(Enum)]
/// enum Foo {
///     Bar,
///     Baz,
///     #[name = "Contains Spaces"]
///     ContainsSpaces,
/// }
/// ```
pub trait Enum {
    /// The human readable names for the variants. These are displayed in the GUI or parameter list,
    /// and also used for parsing text back to a parameter value. The length of this slice
    /// determines how many variants there are.
    fn variants() -> &'static [&'static str];

    /// Get the variant index (which may not be the same as the discriminator) corresponding to the
    /// active variant. The index needs to correspond to the name in [Self::variants()].
    fn to_index(self) -> usize;

    /// Get the variant corresponding to the variant with the same index in [Self::variants()]. This
    /// must always return a value. If the index is out of range, return the first variatn.
    fn from_index(index: usize) -> Self;
}

/// An [IntParam]-backed categorical parameter that allows convenient conversion to and from a
/// simple enum. This enum must derive the re-exported [Enum] trait. Check the trait's documentation
/// for more information on how this works.
pub struct EnumParam<T: Enum> {
    /// A type-erased version of this parameter so the wrapper can do its thing without needing to
    /// know about `T`.
    inner: EnumParamInner,

    /// `T` is only used on the plugin side to convert back to an enum variant. Internally
    /// everything works through the variants field on [EnumParamInner].
    _marker: PhantomData<T>,
}

/// The type-erased internals for [EnumParam] so that the wrapper can interact with it. Acts like an
/// [IntParam] but with different conversions from strings to values.
pub struct EnumParamInner {
    /// The integer parameter backing this enum parameter.
    pub(crate) inner: IntParam,
    /// The human readable variant names, obtained from [Enum::variants()].
    variants: &'static [&'static str],
}

impl<T: Enum + Default> Default for EnumParam<T> {
    fn default() -> Self {
        let variants = T::variants();

        Self {
            inner: EnumParamInner {
                inner: IntParam {
                    value: T::default().to_index() as i32,
                    range: Range::Linear {
                        min: 0,
                        max: variants.len() as i32 - 1,
                    },
                    ..Default::default()
                },
                variants,
            },
            _marker: PhantomData,
        }
    }
}

impl<T: Enum> Display for EnumParam<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl Display for EnumParamInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.variants[self.inner.plain_value() as usize])
    }
}

impl<T: Enum> Param for EnumParam<T> {
    type Plain = T;

    fn update_smoother(&mut self, sample_rate: f32, reset: bool) {
        self.inner.update_smoother(sample_rate, reset)
    }

    fn set_from_string(&mut self, string: &str) -> bool {
        self.inner.set_from_string(string)
    }

    fn plain_value(&self) -> Self::Plain {
        T::from_index(self.inner.plain_value() as usize)
    }

    fn set_plain_value(&mut self, plain: Self::Plain) {
        self.inner.set_plain_value(T::to_index(plain) as i32)
    }

    fn normalized_value(&self) -> f32 {
        self.inner.normalized_value()
    }

    fn set_normalized_value(&mut self, normalized: f32) {
        self.inner.set_normalized_value(normalized)
    }

    fn normalized_value_to_string(&self, normalized: f32, include_unit: bool) -> String {
        self.inner
            .normalized_value_to_string(normalized, include_unit)
    }

    fn string_to_normalized_value(&self, string: &str) -> Option<f32> {
        self.inner.string_to_normalized_value(string)
    }

    fn preview_normalized(&self, plain: Self::Plain) -> f32 {
        self.inner.preview_normalized(T::to_index(plain) as i32)
    }

    fn preview_plain(&self, normalized: f32) -> Self::Plain {
        T::from_index(self.inner.preview_plain(normalized) as usize)
    }

    fn as_ptr(&self) -> ParamPtr {
        self.inner.as_ptr()
    }
}

impl Param for EnumParamInner {
    type Plain = i32;

    fn update_smoother(&mut self, sample_rate: f32, reset: bool) {
        self.inner.update_smoother(sample_rate, reset)
    }

    fn set_from_string(&mut self, string: &str) -> bool {
        match self.variants.iter().position(|n| n == &string) {
            Some(idx) => {
                self.inner.set_plain_value(idx as i32);
                true
            }
            None => false,
        }
    }

    fn plain_value(&self) -> Self::Plain {
        self.inner.plain_value()
    }

    fn set_plain_value(&mut self, plain: Self::Plain) {
        self.inner.set_plain_value(plain)
    }

    fn normalized_value(&self) -> f32 {
        self.inner.normalized_value()
    }

    fn set_normalized_value(&mut self, normalized: f32) {
        self.inner.set_normalized_value(normalized)
    }

    fn normalized_value_to_string(&self, normalized: f32, _include_unit: bool) -> String {
        let index = self.preview_plain(normalized);
        self.variants[index as usize].to_string()
    }

    fn string_to_normalized_value(&self, string: &str) -> Option<f32> {
        self.variants
            .iter()
            .position(|n| n == &string)
            .map(|idx| self.preview_normalized(idx as i32))
    }

    fn preview_normalized(&self, plain: Self::Plain) -> f32 {
        self.inner.preview_normalized(plain)
    }

    fn preview_plain(&self, normalized: f32) -> Self::Plain {
        self.inner.preview_plain(normalized)
    }

    fn as_ptr(&self) -> ParamPtr {
        ParamPtr::EnumParam(self as *const EnumParamInner as *mut EnumParamInner)
    }
}

impl<T: Enum + 'static> EnumParam<T> {
    /// Build a new [Self]. Use the other associated functions to modify the behavior of the
    /// parameter.
    pub fn new(name: &'static str, default: T) -> Self {
        let variants = T::variants();

        Self {
            inner: EnumParamInner {
                inner: IntParam {
                    value: T::to_index(default) as i32,
                    range: Range::Linear {
                        min: 0,
                        max: variants.len() as i32 - 1,
                    },
                    name,
                    ..Default::default()
                },
                variants,
            },
            _marker: PhantomData,
        }
    }

    /// Run a callback whenever this parameter's value changes. The argument passed to this function
    /// is the parameter's new value. This should not do anything expensive as it may be called
    /// multiple times in rapid succession, and it can be run from both the GUI and the audio
    /// thread.
    pub fn with_callback(mut self, callback: Arc<dyn Fn(T) + Send + Sync>) -> Self {
        self.inner.inner.value_changed = Some(Arc::new(move |value| {
            callback(T::from_index(value as usize))
        }));
        self
    }

    /// Get the active enum variant.
    pub fn value(&self) -> T {
        self.plain_value()
    }
}

impl EnumParamInner {
    /// Get the number of variants for this enum.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.variants.len()
    }
}
