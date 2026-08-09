## Comparison to anyhow

Use thiserror if you care about designing your own dedicated error type(s) so
that the caller receives exactly the information that you choose in the event of
failure. This most often applies to library-like code. Use [Anyhow] if you don't
care what error type your functions return, you just want it to be easy. This is
common in application-like code.

[Anyhow]: https://github.com/dtolnay/anyhow

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
