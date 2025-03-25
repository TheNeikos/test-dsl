# test-dsl is a test-helper library to write your own DSLs

It allows you define a set of verbs and conditions, to more easily concentrate
on authoring tests thus making your software more robust.

## How to use


```kdl
testcase "Check for invalid output" {
    connect_to_server 1
    send_message "Hello"
    verify_now {
        received_message "World"
    }
    wait_until_true {

    }
    repeat 10 {
        send_messages "Hello"
    }
}
```

```rust
let mut dsl = TestDsl::<TestHarness>::new();

dsl.register_verb("connect_to_server", |id: usize| {
    // Do stuff!
});

dsl.register_verb("send_message", |msg: String| {
    // Send to server!?
});

dsl.register_verb("repeat", meta_verb(|args, nodes| {

}));

dsl.register_condition("received_message", |mode: WaitingMode, msg: String| {
    if mode == WaitingMode::Wait {
        // Await until the message has been received (if its hasnt already)
    } else {
        // Just check now and return imediately
    }
});

let runner = dsl.load_file("./foobar");

runner.run(TestHarness);
```
