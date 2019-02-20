# batchcensor

This is a small utility to read a configuration file with audio files to censor, and apply a censoring policy to them.

## Installation

First you'll need Rust.

Afterwards you can install batchcensor using `cargo`:

```bash
cargo install batchcensor
```

Make sure that your `.cargo/bin` directory is in your `PATH`.
On Windows this would be: `C:\Users\<username>\.cargo\bin`.

## Configurations

See the [batchcensor-configs](https://github.com/udoprog/batchcensor-configs) project.

## Example Configuration

The following is an example configuration:

```yaml
dirs:
- path: trv1
- path: ar2
  file_prefix: AR2_
  file_extension: wav
  files:
  - path: AAAA_01
  - path: AAAA_02
  - path: ABAA_01
    replace:
    - kind: fuck
      range: "00.876-01.199"
```

This will scan through a directory called `ar2`, looking for files prefixed with `AR2_` with the extension `.wav`.

So for example `ar2/AR2_AAAA_01.wav` would be whitelisted, while a segment of `ar2/AR2_ABAA_01.wav` would be censored.

Note that any file which does not match the configuration in the directory will be completely muted.