# Test assets crate

Download test assets, managing them outside of git.

[crates.io](https://crates.io/crates/test-assets)

[Documentation](https://docs.rs/test-assets/0.2.0/)

git is nice for many purposes, but it stores the whole history.
Once added, an asset will bloat your git repository even after
you've removed it again, until drastic measures are taken like
a rewrite of the history. Best don't even add it to git in the
first place, and keep git to text files only.

Submodules can be used to manage this as well, but they are
very hard to use and add lots of complexity to your users.

With this library, the only thing your users have to do is `git clone`.

## License

Licensed under Apache 2 or MIT (at your option). For details, see the [LICENSE](LICENSE) file.

All examples inside the `examples/` folder are licensed under the
[CC-0](https://creativecommons.org/publicdomain/zero/1.0/) license.

### License of your contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license,
shall be dual licensed / CC-0 licensed as above, without any additional terms or conditions.
