//! test

use test_dsl::TestDsl;
use test_dsl::named_parameters_verb;

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
        "test2",
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

    dsl.add_verb(
        "test3",
        named_parameters_verb!(|_harness: &mut (),
                                _name: String,
                                _pi: String,
                                _pi1: String,
                                _pi2: String| { todo!() }),
    );
}
