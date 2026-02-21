# cfn-teleport

[![Release](https://img.shields.io/github/v/release/udondan/cfn-teleport)][latest]
[![crates.io](https://img.shields.io/badge/crates.io-cfn--teleport-yellowgreen)][crate]
[![License](https://img.shields.io/github/license/udondan/cfn-teleport)][license]

A command-line tool for managing CloudFormation resources across and within stacks.

**Features:**

- **Move resources between stacks** - Transfer resources from one CloudFormation stack to another
- **Rename resources within a stack** - Change logical IDs of resources in the same stack
- **Automatic reference updates** - All CloudFormation references (`Ref`, `Fn::GetAtt`, `Fn::Sub`, `DependsOn`, etc.) are automatically updated

![Demo](https://raw.githubusercontent.com/udondan/cfn-teleport/main/docs/demo.gif)

## Installation

On MacOS and Linux you can install via [Homebrew](https://brew.sh/):

```bash
brew install udondan/software/cfn-teleport
```

On Arch Linux you can install from [AUR](https://aur.archlinux.org/packages/cfn-teleport), e.g.:

```bash
yay -S cfn-teleport
```

On Windows you can install via [Chocolatey](https://community.chocolatey.org/packages/cfn-teleport):

```powershell
choco install cfn-teleport
```

Pre-compiled binaries for various operating systems and architectures are [available for download][latest].

If you have [rust/cargo installed](https://doc.rust-lang.org/cargo/getting-started/installation.html), you can install the [crate]:

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
      --mode <MODE>             Operation mode: 'refactor' (safe, atomic, fewer resource types) or 'import' (legacy, more resource types, can orphan resources) [default: refactor]
  -h, --help                    Print help
  -V, --version                 Print version
```

### Moving Resources Between Stacks

Transfer resources from one stack to another:

```bash
cfn-teleport --source Stack1 --target Stack2 --resource Bucket21D68F7E8 --resource Bucket182C536A1 --yes
```

#### Operation Modes

cfn-teleport supports two modes for cross-stack resource moves:

##### Refactor Mode (Default, Recommended)

Uses the AWS CloudFormation Stack Refactoring API:

```bash
cfn-teleport --source Stack1 --target Stack2 --resource MyBucket --mode refactor
```

**Advantages:**
- ✅ **Safe and atomic** - Either succeeds completely or rolls back with no changes
- ✅ **No orphaned resources** - Resources never end up outside of any stack
- ✅ **Updates resource tags** - `aws:cloudformation:*` tags reflect new stack ownership

**Limitations:**
- ❌ **Fewer supported resource types** - Some resources (like `AWS::EC2::KeyPair`) cannot be moved because updating their tags requires resource replacement

##### Import Mode (Legacy)

Uses the legacy import/export flow (6-step manual process):

```bash
cfn-teleport --source Stack1 --target Stack2 --resource MyKeyPair --mode import
```

**Advantages:**
- ✅ **More resource types** - Can move resources like `AWS::EC2::KeyPair` that don't allow tag updates
- ✅ **No tag updates required** - Only updates CloudFormation's internal tracking database

**Risks:**
- ⚠️ **Can orphan resources** - If the operation fails mid-way (steps 5-6), resources may be left outside any stack
- ⚠️ **Not atomic** - Multi-step process that can leave stacks in inconsistent state on failure
- ⚠️ **Outdated tags** - Resource tags still reference old stack (cosmetic issue only)

**When to use import mode:**
- You need to move a resource type that refactor mode doesn't support
- You understand and accept the risk of potential resource orphaning
- You have a backup/recovery plan if the operation fails

The tool will:

1. Export resources from the source stack (refactor mode: atomic; import mode: manual steps)
2. Import them into the target stack
3. Update all references in both stacks automatically
4. Preserve the physical resources (no deletion/recreation)

### Renaming Resources Within a Stack

Rename resources in the same stack by specifying the same source and target:

```bash
cfn-teleport --source MyStack --target MyStack --resource OldBucketName:NewBucketName --yes
```

**Note:** Same-stack operations always use refactor mode (safe and atomic) regardless of the `--mode` parameter.

The tool will:

1. Rename the logical ID of the resource
2. Update all references (`Ref`, `Fn::GetAtt`, `Fn::Sub`, `DependsOn`, etc.) automatically
3. Preserve the physical resource (no deletion/recreation)

### Interactive Mode

If any of the required options is undefined, the program will prompt for input interactively:

```bash
cfn-teleport
# Will prompt for:
# - Source stack name
# - Target stack name
# - Resources to move/rename
# - Optional: New logical IDs for each resource
```

## Contributing

Contributions are welcome!

This project uses [conventional commits](https://www.conventionalcommits.org/). Please make sure all your merge request titles follow these specifications.

[license]: https://github.com/udondan/iam-floyd/blob/main/LICENSE
[crate]: https://crates.io/crates/cfn-teleport
[latest]: https://github.com/udondan/cfn-teleport/releases/latest
