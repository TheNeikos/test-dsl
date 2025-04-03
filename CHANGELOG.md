# CHANGELOG

<!-- generated from cargo-changelog -->

## v0.4.0

### "Bugfix"

#### Fix infinite macro expansion bug with unexpected token inputs

See 937217f665c6bba8d597e099a5640fac2a8036ea for an explanation
### "Misc"

#### Renamed Condition-related types to make more sense

## v0.3.0

### "Feature"

#### Allow more kinds of inputs rather just NamedSource

#### (#1) Add getter for test case source

#### Refactor internals to allow for more flexible usage

Users can now create their own top-level combinators
#### Separate parsing from running a test

Previously both steps were done at once. This has been separated into two
phases, which in turn allows for more flexibility when it comes to providing
different input schemes. Users can still use same `FunctionVerb` etc.
combinators, but creating your own is now much more accessible.
#### Add named_parameters_verb macro

With this macro, users will be able define verbs from closures that have named parameters.

This means, one can now define verbs such as:

```kdl
connect_to_server ip="127.0.0.1" port=12311
```
#### Add floats and bool as possible arguments

## v0.2.0

### "Misc"

#### Add documentation to all items

#### Remove accidentally public TestCase::new method

## v0.1.0

### "Feature"

#### Implemented first version

This release implements a barebones version that includes the following verbs:

- `repeat n { .. }` to repeat a given list of verbs
- `group { .. }` to group together a list of verbs
- `assert { .. }` to assert a list of conditions
