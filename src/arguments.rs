pub trait VerbArgument: Sized + 'static {
    const TYPE_NAME: &'static str;
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

