# error-accumulator

[![Crates.io](https://img.shields.io/crates/v/error-accumulator.svg)](https://crates.io/crates/error-accumulator)
[![Docs.rs](https://docs.rs/error-accumulator/badge.svg)](https://docs.rs/error-accumulator)
[![CI](https://github.com/MattesWhite/error-accumulator/workflows/CI/badge.svg)](https://github.com/MattesWhite/error-accumulator/actions)


This crate provides utility to make it easier for developers to write input validation that:

1. Tries to find as many errors as possible before returning.
2. Not only validates but also converts input so we get a types output that uphold the checked invariants.

## Roadmap

Currently, the crate provides the basic API that I wanted to provide with most features ready to use. However, there are some bigger features I want to implement before a potential v1.0 release.

### Parser trait

The crate should provide a trait to validate and convert one type into another while accumulating errors.
This way it becomes easier to re-use the code, e.g. in nested structs.

However, there are a number of open questions how the trait and its methods should look like and how to handle structs, arrays and fields.
In order to prevent to many breaking changes I need more time to figure out the best API design.

### Derive macro

Once a stable trait is available I want to implement a derive macro so users have less boilerplate code to write.
But again there are many options how this can be implemented and how types that don't implement the crate's trait should be handled.
How can common parsing traits like `FromStr` or `TryFrom` be leveraged?
What are the best defaults? Etc.

## Motivation

Imagine you want to enter a new password on a website.
On first try you get a 'password must contain capital letters'.
On the next try you get 'password must contain numbers'.
On the third try you get 'passowrd must contain special characters'.
This is not to uncommon and many people know what I'm talking about.
Everywhere user input must be validated, be it websites, config files or others,
  it is very common that validation stops at the first try and returns that error immediately.
Rust makes this even easier by just using the ?-operator.

The example `requester` does the same thing try it out and get it running with the provided `config.yaml`:

```shell
cargo run --example requester -- -c examples/config.yaml
```

<details>
<summary>
Example output
</summary>

```
> cargo run --example requester --quiet -- -c examples/config.yaml
Error: unknown time unit "sek", supported units: ns, us/µs, ms, sec, min, hours, days, weeks, months, years (and few variations)

Location:
    examples/requester.rs:86:24
```

After unit fix:

```
> cargo run --example requester --quiet -- -c examples/config.yaml
Error: invalid StatusCode

Caused by:
    invalid status code

Location:
    examples/requester.rs:92:22
```

After status code fix:

```
> cargo run --example requester --quiet -- -c examples/config.yaml
Error: invalid URL

Caused by:
    relative URL without a base

Location:
    examples/requester.rs:96:30
```

After URL fix:

```
> cargo run --example requester --quiet -- -c examples/config.yaml
Error: error sending request for url (http://no.inter.net/)

Caused by:
   0: client error (Connect)
   1: dns error
   2: failed to lookup address information: Name or service not known

Location:
    examples/requester.rs:67:24
```
</details>

You made it, great! Now reset `config.yaml` and run the example again with `--accumulate` to use the utilities provided by this crate:

```shell
cargo run --example requester -- -c examples/config.yaml --accumulate
```

Output:

```
Error: Accumulated errors:
- interval: unknown time unit "sek", supported units: ns, us/µs, ms, sec, min, hours, days, weeks, months, years (and few variations)
- hosts[0].url: error sending request for url (http://no.inter.net/)
- hosts[2].url: relative URL without a base
- hosts[2].expected_status: invalid status code


Location:
    examples/requester.rs:60:9
```

- All errors at once.
- Paths to where the errors originated.

Great!

## Similar crates

A crate with a similar focus and purpose is the popular [validator crate](https://crates.io/crates/validator).
It's design was also one of the major inspirations for error-accumulator.
Validator is way more mature than error-accumulator and comes with a derive macro,
  however, it focuses on validation only.
The issue is that [`Validator::validate()`](https://docs.rs/validator/latest/validator/trait.Validate.html#tymethod.validate) takes the input only by reference.
As a result, a user can't know when receiving an instance of a type implementing `Validator` if the validation was already called or not,
  opening the possibility of not calling `validate()` at all.
In contrast, error-accumulator combines validation and conversion into one step, following the idea of 'parse don't validate',
  so it's easier to ensure at compile time that data was already checked for validity and that invariants are enforced.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
