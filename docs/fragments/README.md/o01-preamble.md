derive(Error)
=============

[<img alt="github" src="https://img.shields.io/badge/github-dtolnay/thiserror-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/dtolnay/thiserror)
[<img alt="crates.io" src="https://img.shields.io/crates/v/thiserror.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/thiserror)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-thiserror-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/thiserror)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/dtolnay/thiserror/ci.yml?branch=master&style=for-the-badge" height="20">](https://github.com/dtolnay/thiserror/actions?query=branch%3Amaster)

This library provides a convenient derive macro for the standard library's
[`std::error::Error`] trait.

[`std::error::Error`]: https://doc.rust-lang.org/std/error/trait.Error.html

```toml
[dependencies]
thiserror = "2"
```

<br>
