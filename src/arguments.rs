//! Traits related to arguments of verbs and conditions

/// A type that can be used as an argument of Verbs and Conditions
pub trait VerbArgument: Sized + 'static {
    /// A human-readable typename
    ///
    /// This is shown only in error-messages
    const TYPE_NAME: &'static str;

    /// Convert from a [`KdlEntry`](kdl::KdlEntry) to the value
    ///
    /// Implementations are free to accept more than a single way of interpreting values. E.g. a
    /// string and a integer.
    fn from_value(value: &kdl::KdlEntry) -> Option<Self>;
}

impl VerbArgument for String {
    const TYPE_NAME: &'static str = "string";
    fn from_value(value: &kdl::KdlEntry) -> Option<Self> {
        value.value().as_string().map(ToOwned::to_owned)
    }
}

impl VerbArgument for usize {
    const TYPE_NAME: &'static str = "integer";
    fn from_value(value: &kdl::KdlEntry) -> Option<Self> {
        value.value().as_integer().map(|i| i as usize)
    }
}
