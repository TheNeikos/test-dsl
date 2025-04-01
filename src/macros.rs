#[rustfmt::skip]
macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!([], T1);
        $name!([T1], T2);
        $name!([T1, T2], T3);
        $name!([T1, T2, T3], T4);
        $name!([T1, T2, T3, T4], T5);
        $name!([T1, T2, T3, T4, T5], T6);
        $name!([T1, T2, T3, T4, T5, T6], T7);
        $name!([T1, T2, T3, T4, T5, T6, T7], T8);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8], T9);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9], T10);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10], T11);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11], T12);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12], T13);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13], T14);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14], T15);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15], T16);
    };
}

/// Define a new type that implements [`ParseArguments`](crate::arguments::ParseArguments)
///
/// This can then be used in your custom [`Verb`](crate::Verb) or [`TestCondition`](crate::condition::TestCondition) implementations.
#[macro_export]
macro_rules! named_parameters {
    ( $vis:vis $param_name:ident { $($key:ident = $value:ty),* $(,)? }) => {
        $vis struct $param_name {
            $($key: $value),*
        }

        impl<H> $crate::arguments::ParseArguments<H> for $param_name {
            fn parse(_: &$crate::TestDsl<H>, node: &$crate::kdl::KdlNode) -> Result<Self, $crate::error::TestErrorCase> {
                $(
                    let $key: $value = $crate::arguments::VerbArgument::from_value(node.entry(stringify!($key)).unwrap()).unwrap();
                )*

                Ok($param_name {
                    $(
                        $key
                    ),*
                })
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::TestDsl;
    use crate::arguments::ParseArguments;

    #[test]
    fn simple_kv() {
        named_parameters!(CoolIntegers {
            pi = usize,
            name = String
        });

        let dsl = TestDsl::<()>::new();

        let node = kdl::KdlNode::parse("foo pi=4 name=PI { other stuff }").unwrap();

        let ints = CoolIntegers::parse(&dsl, &node).unwrap();

        assert_eq!(ints.pi, 4);
        assert_eq!(ints.name, "PI");
    }
}
