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

/// Define a new type that implements [`ParseArguments`](crate::argument::ParseArguments)
///
/// This can then be used in your custom [`Verb`](crate::Verb) or [`Condition`](crate::condition::Condition) implementations.
///
/// **Note:** The definition uses `=` instead of the usual `:` to delimit fields and their types.
/// This is on purpose, as this may later be expanded to allow for positional arguments as well.
///
/// ```
/// use test_dsl::named_parameters;
///
/// named_parameters! {
///     Frobnicator {
///         foo = usize,
///         name = String
///     }
/// }
/// ```
#[macro_export]
macro_rules! named_parameters {
    ( $vis:vis $param_name:ident { $($key:ident = $value:ty),* $(,)? }) => {
        #[derive(Debug, Clone)]
        $vis struct $param_name {
            $($key: $value),*
        }

        impl<H> $crate::argument::ParseArguments<H> for $param_name {
            fn parse(_: &$crate::TestDsl<H>, node: &$crate::kdl::KdlNode) -> Result<Self, $crate::error::TestErrorCase> {
                $(
                    let $key: $value = $crate::argument::VerbArgument::from_value(node.entry(stringify!($key)).unwrap()).unwrap();
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

#[macro_export]
#[cfg(not(doc))]
#[expect(missing_docs, reason = "This is documented further below")]
macro_rules! named_parameters_verb {
    ($($input:tt)*) => {{
        $crate::__inner_named_parameters_verb!(@impl { $($input)* })
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __inner_named_parameters_verb {
    (@impl { |$name:ident : &mut $ty:ty $(, $param_name:ident : $param_type:ty)* $(,)?| $rest:block }) => {{
        #[derive(Debug, Clone)]
        struct __NamedVerb {
            $($param_name : $param_type),*
        }

        let verb = $crate::verb::FunctionVerb::<_, __NamedVerb>::new({
            #[derive(Clone)]
            struct __Caller;

            impl $crate::verb::CallableVerb<$ty, __NamedVerb> for __Caller {
                fn call(&self, harness: &mut $ty, node: &__NamedVerb) -> $crate::miette::Result<()> {

                    let verb = |$name : &mut $ty $(, $param_name : $param_type)*,| {
                        $rest
                    };

                    $(
                        let $param_name: $param_type = node.$param_name.clone();
                    )*

                    verb(harness, $($param_name),*)
                }
            }

            __Caller
        });

        impl<H> $crate::argument::ParseArguments<H> for __NamedVerb {
            fn parse(_: &$crate::TestDsl<H>, node: &$crate::kdl::KdlNode) -> Result<Self, $crate::error::TestErrorCase> {
                $(
                    let $param_name: $param_type = $crate::argument::VerbArgument::from_value(node.entry(stringify!($param_name)).unwrap()).unwrap();
                )*

                Ok({
                    __NamedVerb {
                        $($param_name),*
                    }
                })
            }
        }

        verb
    }};
}

/// Define a verb using a closure, where the argument names are used as the key names
///
/// ```
/// # use test_dsl::{TestDsl, named_parameters_verb};
/// let mut dsl = TestDsl::<()>::new();
///
/// dsl.add_verb(
///     "test",
///     named_parameters_verb!(|_harness: &mut (), name: String, pi: usize| {
///         println!("{name} = {pi}");
///         Ok(())
///     }),
/// );
/// ```
#[cfg(doc)]
#[macro_export]
macro_rules! named_parameters_verb {
    ($($input:tt)*) => {};
}

#[cfg(test)]
mod tests {
    use crate::TestDsl;
    use crate::argument::ParseArguments;

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

    #[test]
    fn simple_named_closure() {
        let mut dsl = TestDsl::<()>::new();

        dsl.add_verb(
            "test",
            named_parameters_verb!(|_harness: &mut (), name: String, pi: usize| {
                println!("{name} = {pi}");
                Ok(())
            }),
        );

        dsl.add_verb(
            "test_many",
            named_parameters_verb!(|_harness: &mut (),
                                    _name: String,
                                    _pi: usize,
                                    _pi1: usize,
                                    _pi2: usize,
                                    _pi3: usize,
                                    _pi4: usize,
                                    _pi5: usize,
                                    _pi6: usize,
                                    _pi7: usize,
                                    _pi8: usize,
                                    _pi9: usize,
                                    _pi10: usize,
                                    _pi11: usize,
                                    _pi12: usize,
                                    _pi13: usize,
                                    _pi14: usize,
                                    _pi15: usize,
                                    _pi16: usize,
                                    _pi17: usize,
                                    _pi18: usize,
                                    _pi19: usize,
                                    _pi20: usize,
                                    _pi21: usize,
                                    _pi22: usize,
                                    _pi23: usize,
                                    _pi24: usize| { Ok(()) }),
        );
    }
}
