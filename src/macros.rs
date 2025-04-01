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

/// Define a verb using a closure, where the argument names are used as the key names
#[macro_export]
macro_rules! named_verb {
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

    (@call $struct_name:ident => { |$_name:ident : &mut $ty:ty $(, $($_:tt)*)? } => $($rest:tt)*) => {{
        #[derive(Clone)]
        struct __Caller;

        impl $crate::verb::CallableVerb<$ty, $struct_name> for __Caller {
            fn call(&self, harness: &mut $ty, node: &$struct_name) -> $crate::miette::Result<()> {

                let verb = $($rest)*;

                $crate::named_verb!(@extract node => $($rest)*);

                $crate::named_verb!(@verb_params verb harness => $($rest)*)
            }
        }

        __Caller
    }};

    (@define $name:ident => $($input:tt)*) => {{

        $crate::named_verb!(@define_args $name => { $($input)* });

        impl<H> $crate::arguments::ParseArguments<H> for $name {
            fn parse(_: &$crate::TestDsl<H>, node: &$crate::kdl::KdlNode) -> Result<Self, $crate::error::TestErrorCase> {

                $crate::named_verb!(@parse_args node => $($input)*);

                Ok($crate::named_verb!(@get_args $name => $($input)*))
            }
        }

        $crate::verb::FunctionVerb::<_, $name>::new(

                $crate::named_verb!(@call $name => { $($input)* } => $($input)*)
        )
    }};

    ($($input:tt)*) => {
        $crate::named_verb!(@define __NamedArgs => $($input)*)
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

    #[test]
    fn simple_named_closure() {
        let mut dsl = TestDsl::<()>::new();

        dsl.add_verb(
            "test",
            named_verb!(|_harness: &mut (), name: String, pi: usize| {
                println!("{name} = {pi}");
                Ok(())
            }),
        );
    }
}
