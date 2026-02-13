# completions

Generate shell completion scripts for the `tealeaf` CLI.

## Usage

```bash
tealeaf completions <SHELL>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<SHELL>` | Target shell: `bash`, `zsh`, `fish`, `powershell`, or `elvish` |

## Installation

### Bash

```bash
tealeaf completions bash > ~/.local/share/bash-completion/completions/tealeaf
```

### Zsh

```bash
tealeaf completions zsh > ~/.zsh/completions/_tealeaf
# Then add ~/.zsh/completions to your fpath in ~/.zshrc:
# fpath=(~/.zsh/completions $fpath)
```

### Fish

```bash
tealeaf completions fish > ~/.config/fish/completions/tealeaf.fish
```

### PowerShell

```powershell
tealeaf completions powershell > tealeaf.ps1
# Then add to your PowerShell profile:
# . path\to\tealeaf.ps1
```

### Elvish

```bash
tealeaf completions elvish > ~/.config/elvish/lib/tealeaf.elv
```

## What Gets Completed

Once installed, tab completion works for:

- **Subcommands**: `tealeaf com<TAB>` completes to `tealeaf compile`
- **Flags**: `tealeaf compile --<TAB>` shows `--output` and `--help`
- **Subcommand help**: `tealeaf help <TAB>` lists all subcommands
