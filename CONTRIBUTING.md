# Contributing to Insta

Thanks for your interest in contributing to Insta! Insta welcomes contribution
from everyone in the form of suggestions, bug reports, pull requests, and feedback.
This document gives some guidance if you are thinking of helping out.

## Submitting Bug Reports and Feature Requests

When reporting a bug or asking for help, please include enough details so that
others helping you can reproduce the behavior you are seeing.

Opening an issue is as easy. Just [follow this link](https://github.com/mitsuhiko/insta/issues/new/choose) and fill out the fields in the appropriate provided template.

When making a feature request, please make it clear what problem you intend to
solve with the feature and maybe provide some ideas for how to go about that.

## Running the Tests

To run all tests a makefile is provided

```sh
make test
```

To format the code, run:

```sh
make format
```

To run the version of `cargo-insta` in the working directory, run:

```sh
cargo run -p cargo-insta -- test # (or `review` or whatever command you want to run)
```

...in contrast to running `cargo insta`, which invokes the installed version of
`cargo-insta`, and so make iterating more difficult.

## Writing tests

If making non-trivial changes to `cargo-insta`, please add an integration test to
`cargo-insta/tests/main.rs`. Feel free to add an issue if anything is unclear.

## Website / Documentation

To make changes to the website or documentation, have a look at the
separate [insta-website](https://github.com/mitsuhiko/insta-website) repository.

## Conduct

This issue tracker follows the [Rust Code of Conduct]. For escalation or moderation
issues please contact Armin (armin.ronacher@active-4.com) instead of the Rust moderation team.

[rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
