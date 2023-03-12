# WIP cfn-teleport

**This is work-in-progress! There are many thing left to do.**

A command line tool which can migrate CloudFormation resources between stacks.

## Usage

```bash
$ cfn-teleport --help
Migrate CloudFormation resources between stacks

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
