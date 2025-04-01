//! Traits related to arguments of verbs and conditions

use crate::ConditionInstance;
use crate::TestDsl;
use crate::VerbInstance;
use crate::error;
use crate::error::TestErrorCase;

/// Types that can be parsed from a node as arguments
///
/// This includes both named/positional parameters as well as child nodes
pub trait ParseArguments<H>: std::fmt::Debug + Clone + Sized + 'static {
    /// Do the parsing and return an instance
    ///
    /// See [`VerbInstance`] and
    /// [`ConditionInstance`] for how to get an instance from a node.
    fn parse(test_dsl: &TestDsl<H>, node: &kdl::KdlNode) -> Result<Self, TestErrorCase>;
}

pub(crate) trait BoxedArguments<H>: std::fmt::Debug + std::any::Any {
    fn clone_box(&self) -> Box<dyn BoxedArguments<H>>;
    fn as_dyn_any(&self) -> &dyn std::any::Any;
}

impl<H: 'static> Clone for Box<dyn BoxedArguments<H>> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

impl<H, A: ParseArguments<H>> BoxedArguments<H> for A {
    fn clone_box(&self) -> Box<dyn BoxedArguments<H>> {
        Box::new(self.clone())
    }

    fn as_dyn_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl<H> ParseArguments<H> for ((),) {
    fn parse(_test_dsl: &TestDsl<H>, _node: &kdl::KdlNode) -> Result<Self, TestErrorCase> {
        Ok(((),))
    }
}

macro_rules! impl_parse_arguments {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(non_snake_case, unused_mut)]
        impl<H, $($ty,)* $last> ParseArguments<H> for ($($ty,)* $last,)
            where
                $( $ty: VerbArgument + 'static , )*
                $last: VerbArgument + 'static,
                ($($ty,)* $last,): std::fmt::Debug,
        {
            fn parse(_test_dsl: &TestDsl<H>, node: &kdl::KdlNode) -> Result<Self, TestErrorCase> {
                let mut args = node.iter();

                let total_count = 1
                    $(
                        + {
                            const _: () = {
                                #[allow(unused)]
                                let $ty = ();
                            };
                            1
                        }

                    )*;

                let mut running_count = 1;

                $(
                    let arg = args.next().ok_or_else(|| TestErrorCase::MissingArgument {
                        parent: node.span(),
                        missing: format!("This verb takes {} arguments, you're missing the {}th argument.", total_count, running_count),
                    })?;

                    let $ty = <$ty as VerbArgument>::from_value(arg).ok_or_else(|| {
                        TestErrorCase::WrongArgumentType {
                            parent: node.name().span(),
                            argument: arg.span(),
                            expected: format!("This verb takes a '{}' as its argument here.", <$ty as VerbArgument>::get_error_type_name()),
                        }
                    })?;
                    running_count += 1;
                )*

                let _ = running_count;

                let arg = args.next().ok_or_else(|| TestErrorCase::MissingArgument {
                    parent: node.span(),
                    missing: format!("This verb takes {tc} arguments, you're missing the {tc}th argument.", tc = total_count),
                })?;
                let $last = <$last as VerbArgument>::from_value(arg).ok_or_else(|| {
                    TestErrorCase::WrongArgumentType {
                        parent: node.name().span(),
                        argument: arg.span(),
                        expected: format!("This verb takes a '{}' as its argument here.", <$last as VerbArgument>::get_error_type_name()),
                    }
                })?;


                Ok(($($ty,)* $last,))
            }
        }
    };
}

all_the_tuples!(impl_parse_arguments);

/// A type that can be used as an argument of Verbs and Conditions
pub trait VerbArgument: Clone {
    /// A human-readable typename
    ///
    /// This is shown only in error-messages
    fn get_error_type_name() -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Convert from a [`KdlEntry`](kdl::KdlEntry) to the value
    ///
    /// Implementations are free to accept more than a single way of interpreting values. E.g. a
    /// string and a integer.
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

impl VerbArgument for f64 {
    fn from_value(value: &kdl::KdlEntry) -> Option<Self> {
        value.value().as_float()
    }
}

impl VerbArgument for bool {
    fn from_value(value: &kdl::KdlEntry) -> Option<Self> {
        value.value().as_bool()
    }
}

/// Parameters with a list of nodes that are conditions
pub struct ConditionChildren<H, A> {
    parameters: A,
    children: Vec<ConditionInstance<H>>,
}

impl<H, A: std::fmt::Debug> std::fmt::Debug for ConditionChildren<H, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConditionChildren")
            .field("parameters", &self.parameters)
            .field("children", &self.children)
            .finish()
    }
}

impl<H: 'static, A: Clone> Clone for ConditionChildren<H, A> {
    fn clone(&self) -> Self {
        Self {
            parameters: self.parameters.clone(),
            children: self.children.clone(),
        }
    }
}

impl<H, A> ConditionChildren<H, A> {
    /// Get the parameters
    pub fn parameters(&self) -> &A {
        &self.parameters
    }

    /// Get the children nodes
    pub fn children(&self) -> &[ConditionInstance<H>] {
        &self.children
    }
}

impl<H: 'static, A: ParseArguments<H>> ParseArguments<H> for ConditionChildren<H, A> {
    fn parse(test_dsl: &TestDsl<H>, node: &kdl::KdlNode) -> Result<Self, error::TestErrorCase> {
        let arguments = A::parse(test_dsl, node)?;

        let children = node
            .iter_children()
            .map(|node| ConditionInstance::with_test_dsl(test_dsl, node))
            .collect::<Result<_, _>>()?;

        Ok(ConditionChildren {
            parameters: arguments,
            children,
        })
    }
}

/// Parameters with a list of nodes that are verbs
pub struct VerbChildren<H, A> {
    parameters: A,
    children: Vec<VerbInstance<H>>,
}

impl<H, A: std::fmt::Debug> std::fmt::Debug for VerbChildren<H, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerbChildren")
            .field("parameters", &self.parameters)
            .field("children", &self.children)
            .finish()
    }
}

impl<H: 'static, A: Clone> Clone for VerbChildren<H, A> {
    fn clone(&self) -> Self {
        Self {
            parameters: self.parameters.clone(),
            children: self.children.clone(),
        }
    }
}

impl<H, A> VerbChildren<H, A> {
    /// Get the parameters
    pub fn parameters(&self) -> &A {
        &self.parameters
    }

    /// Get the children
    pub fn children(&self) -> &[VerbInstance<H>] {
        &self.children
    }
}

impl<H: 'static, A: ParseArguments<H>> ParseArguments<H> for VerbChildren<H, A> {
    fn parse(test_dsl: &TestDsl<H>, node: &kdl::KdlNode) -> Result<Self, error::TestErrorCase> {
        let arguments = A::parse(test_dsl, node)?;

        let children = node
            .iter_children()
            .map(|node| VerbInstance::with_test_dsl(test_dsl, node))
            .collect::<Result<_, _>>()?;

        Ok(VerbChildren {
            parameters: arguments,
            children,
        })
    }
}
