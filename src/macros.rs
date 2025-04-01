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

#[macro_export]
#[cfg(not(doc))]
#[expect(missing_docs, reason = "This is documented further below")]
macro_rules! named_parameters_verb {
    (@define_args $struct_name:ident => { |$_name:ident : $_ty:ty $(, $name:ident : $kind:ty)* $(,)?| $rest:block }) => {
        #[derive(Debug, Clone)]
        struct $struct_name {
            $($name : $kind),*
        }
    };

    (@get_args $struct_name:ident => |$_name:ident : $_ty:ty $(, $name:ident : $kind:ty)* $(,)?| $rest:block) => {
        $struct_name {
            $($name),*
        }
    };

    (@parse_args $node:ident => |$_name:ident : $_ty:ty $(, $name:ident : $kind:ty)* $(,)?| $rest:block) => {
        $(
            let $name: $kind = $crate::arguments::VerbArgument::from_value($node.entry(stringify!($name)).unwrap()).unwrap();
        )*
    };

    (@extract $node:ident => |$_name:ident : $_ty:ty $(, $name:ident : $kind:ty)* $(,)?| $rest:block) => {
        $(
            let $name: $kind = $node.$name.clone();
        )*
    };

    (@verb_params $verb:ident $harness:ident => |$_name:ident : $_ty:ty $(, $name:ident : $kind:ty)* $(,)?| $rest:block) => {
        $verb($harness, $($name),*)
    };

    (@call $struct_name:ident => { $($rest:tt)* } => { |$_name:ident : &mut $ty:ty $(,$_:ident : $__:ty)*| $_rest:block }) => {{
        #[derive(Clone)]
        struct __Caller;

        impl $crate::verb::CallableVerb<$ty, $struct_name> for __Caller {
            fn call(&self, harness: &mut $ty, node: &$struct_name) -> $crate::miette::Result<()> {

                let verb = $($rest)*;

                $crate::named_parameters_verb!(@extract node => $($rest)*);

                $crate::named_parameters_verb!(@verb_params verb harness => $($rest)*)
            }
        }

        __Caller
    }};

    ($($input:tt)*) => {{
        let verb = $crate::verb::FunctionVerb::<_, __NamedVerb>::new(
            $crate::named_parameters_verb!(@call __NamedVerb => { $($input)* } => { $($input)* })
        );

        $crate::named_parameters_verb!(@define_args __NamedVerb => { $($input)* });

        impl<H> $crate::arguments::ParseArguments<H> for __NamedVerb {
            fn parse(_: &$crate::TestDsl<H>, node: &$crate::kdl::KdlNode) -> Result<Self, $crate::error::TestErrorCase> {

                $crate::named_parameters_verb!(@parse_args node => $($input)*);

                Ok($crate::named_parameters_verb!(@get_args __NamedVerb => $($input)*))
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
    }
}
