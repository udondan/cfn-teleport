# cfn-teleport

[![Release](https://img.shields.io/github/v/release/udondan/cfn-teleport)][latest]
[![crates.io](https://img.shields.io/badge/crates.io-cfn--teleport-yellowgreen)][crate]
[![License](https://img.shields.io/github/license/udondan/cfn-teleport)][license]

A command-line tool which can move CloudFormation resources between stacks.

![Demo](https://raw.githubusercontent.com/udondan/cfn-teleport/main/docs/demo.gif)

## Installation

On MacOS you can install via [Homebrew](https://brew.sh/):

```bash
brew install udondan/software/cfn-teleport
```

On Arch Linux you can install from [AUR](https://aur.archlinux.org/packages/cfn-teleport), e.g.:

```bash
yay -S cfn-teleport
```

Pre-compiled binaries for various operating systems and architectures are [available for download][latest].

If you have [rust/cargo installed](https://doc.rust-lang.org/cargo/getting-started/installation.html), you can simple install the [crate]:

```bash
cargo install cfn-teleport
```

## Usage

```bash
$ cfn-teleport --help
Move CloudFormation resources between stacks

Usage: cfn-teleport [OPTIONS]

Options:
  -s, --source <SOURCE>         Name of the source stack
  -t, --target <TARGET>         Name of the target stack
  -r, --resource <ID[:NEW_ID]>  Logical ID of a resource from the source stack - optionally with a new ID for the target stack
  -y, --yes                     Automatically confirm all prompts
  -h, --help                    Print help
  -V, --version                 Print version
```

Example usage:

```bash
cfn-teleport --source Stack1 --target Stack2 --resource Bucket21D68F7E8 --resource Bucket182C536A1 --yes
```

If any of the required options is undefined, the program will ask for it during execution.

## Contributing

Contributions are welcome!

This project uses [conventional commits](https://www.conventionalcommits.org/). Please make sure all your merge request titles follow these specifications.

   [license]: https://github.com/udondan/iam-floyd/blob/main/LICENSE
   [crate]: https://crates.io/crates/cfn-teleport
   [latest]: https://github.com/udondan/cfn-teleport/releases/latest
