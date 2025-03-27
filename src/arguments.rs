pub trait VerbArgument: Sized + 'static {
    fn from_value(value: &kdl::KdlEntry) -> Option<Self>;
}

impl VerbArgument for String {
    fn from_value(value: &kdl::KdlEntry) -> Option<Self> {
        value.value().as_string().map(ToOwned::to_owned)
    }
}

impl VerbArgument for usize {
    fn from_value(value: &kdl::KdlEntry) -> Option<Self> {
        value.value().as_integer().map(|i| i as usize)
    }
}

