# RusSOL

This is the implementation of the tool described in the paper [Leveraging Rust Types for Program Synthesis](https://dl.acm.org/doi/abs/10.1145/3591278).

For setup and use, follow the steps that the [CI](https://github.com/JonasAlaif/russol-alpha/blob/main/.github/workflows/ci.yml) takes. Execute with `cargo run /path/to/file.rs`.

Test files can be found [here](https://github.com/JonasAlaif/russol-alpha/tree/main/ruslic/tests), the ones under `synth` work (tested with CI), there are also some under `unsupported` due to known limitations of the search.
