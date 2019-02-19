# batchcensor

This is a small utility to read a configuration file with audio files to censor, and apply a censoring policy to them.

## Configurations

See this [Google Drive](https://drive.google.com/drive/folders/1wRlPdnIT610a6svha-YOntW_iyAY918q?usp=sharing).

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